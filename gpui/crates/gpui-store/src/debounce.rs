//! The debounced settings writer.
//!
//! [`Store::save_settings_debounced`](crate::Store::save_settings_debounced)
//! pushes settings into an unbounded channel; a named background thread keeps
//! only the newest one and writes it once the channel has been quiet for
//! [`DEBOUNCE_INTERVAL`]. The calling thread never opens, writes or renames a
//! file in that path — it only moves a value into the queue.
//!
//! The worker also answers [`Store::flush_debounced`](crate::Store::flush_debounced),
//! which is what shutdown and tests use to wait for the pending write. If the
//! `Store` is dropped with a write still queued, the dropped sender ends the
//! thread's `recv` and it writes the last settings before exiting.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::error::{StoreError, StoreFile};
use crate::file::write_atomic;
use crate::settings::Settings;

/// How long the writer waits for the next edit before committing.
pub const DEBOUNCE_INTERVAL: Duration = Duration::from_millis(500);

const THREAD_NAME: &str = "gpui-store-debounce";

/// What the caller hands the worker. `Flush` carries the reply channel so the
/// wait is on the caller's own receipt, not a sleep.
pub(crate) enum Job {
    Save(Settings),
    Flush(std::sync::mpsc::Sender<Result<(), StoreError>>),
}

/// The owned half of the debounce pair. Dropping it closes the channel, which
/// makes the worker finish its pending write and exit.
pub(crate) struct SettingsWriter {
    sender: std::sync::mpsc::Sender<Job>,
    last_error: Arc<Mutex<Option<StoreError>>>,
}

impl SettingsWriter {
    pub(crate) fn spawn(root: PathBuf) -> Result<Self, StoreError> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let last_error = Arc::new(Mutex::new(None));
        let errors = Arc::clone(&last_error);

        thread::Builder::new()
            .name(THREAD_NAME.to_owned())
            .spawn(move || run(receiver, root, errors))
            .map_err(|source| StoreError::DebouncedWorker { source })?;

        Ok(Self { sender, last_error })
    }

    /// Queue a save. Unbounded channel: never blocks, never does IO.
    pub(crate) fn enqueue(&self, settings: Settings) -> Result<(), StoreError> {
        self.sender
            .send(Job::Save(settings))
            .map_err(|_| StoreError::DebouncedWorkerStopped)
    }

    /// Wait for the queued settings to reach disk. This is the one blocking
    /// call, and it exists for shutdown and tests.
    pub(crate) fn flush(&self) -> Result<(), StoreError> {
        let (reply, answer) = std::sync::mpsc::channel();
        self.sender
            .send(Job::Flush(reply))
            .map_err(|_| StoreError::DebouncedWorkerStopped)?;
        answer.recv().map_err(|_| StoreError::DebouncedWorkerStopped)?
    }

    /// The last background write failure, if no one has taken it yet.
    pub(crate) fn take_error(&self) -> Option<StoreError> {
        lock(&self.last_error).take()
    }
}

/// `Mutex::lock` without the poisoning branch: the worker only stores a value,
/// and a panic elsewhere must not brick the writer.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn write(root: &PathBuf, settings: Option<Settings>) -> Result<(), StoreError> {
    match settings {
        Some(settings) => write_atomic(root, StoreFile::Settings, &settings),
        None => Ok(()),
    }
}

fn run(
    receiver: std::sync::mpsc::Receiver<Job>,
    root: PathBuf,
    last_error: Arc<Mutex<Option<StoreError>>>,
) {
    let mut pending: Option<Settings> = None;

    'outer: loop {
        match receiver.recv() {
            Ok(Job::Save(settings)) => pending = Some(settings),
            Ok(Job::Flush(reply)) => {
                let _ = reply.send(write(&root, pending.take()));
                continue;
            }
            Err(_) => break,
        }

        // Coalesce: only the newest settings matter, and the write happens when
        // the caller has been quiet for one interval.
        loop {
            match receiver.recv_timeout(DEBOUNCE_INTERVAL) {
                Ok(Job::Save(settings)) => pending = Some(settings),
                Ok(Job::Flush(reply)) => {
                    // A flush is a caller waiting on this exact write, so its
                    // error is delivered by the reply, not parked here.
                    let _ = reply.send(write(&root, pending.take()));
                    continue 'outer;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                // The Store is gone; write once more below and stop.
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break 'outer,
            }
        }

        if let Err(error) = write(&root, pending.take()) {
            *lock(&last_error) = Some(error);
        }
    }

    // Channel closed with an edit still queued: do not lose the newest state.
    if let Err(error) = write(&root, pending.take()) {
        *lock(&last_error) = Some(error);
    }
}

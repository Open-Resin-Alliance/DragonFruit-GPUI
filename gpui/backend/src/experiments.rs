//! Experiment gates — `set_experiment_overrides`, and the `is_experiment_enabled`
//! seam.
//!
//! Ported from `src-tauri/src/experiments.rs`. The manifest is embedded at compile
//! time from `src/config/experiments.json`, so the declared experiments and their
//! defaults are the same ones the web build reads.
//!
//! The shell's `set_experiment_overrides` was a command because the frontend pushed
//! the user's toggles in (`src/features/experiments/syncExperimentsToNative.ts`);
//! gated commands then called `is_experiment_enabled` at the top and refused when
//! it was false. Today the chitubox gate is enforced in the frontend, so the seam is
//! public and unused by any command — it is kept because it is the function a future
//! gated Rust command reaches for.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use serde::Deserialize;

const EXPERIMENTS_MANIFEST_JSON: &str = include_str!("../../../../src/config/experiments.json");

/// Compile-time experiment declarations, reduced to what gating needs: the set of
/// known ids and which ones default to enabled.
struct ExperimentManifest {
    known_ids: HashSet<String>,
    default_enabled: HashSet<String>,
}

fn manifest() -> &'static ExperimentManifest {
    static MANIFEST: OnceLock<ExperimentManifest> = OnceLock::new();
    MANIFEST.get_or_init(|| {
        let parsed: RawManifest = match serde_json::from_str(EXPERIMENTS_MANIFEST_JSON) {
            Ok(parsed) => parsed,
            Err(error) => {
                // The manifest is embedded at compile time, so a parse failure means
                // `experiments.json` was hand-edited into an invalid state. Log it and
                // degrade to an empty manifest rather than panicking: every gate
                // closes and only released behaviour runs.
                log::error!(
                    "[experiments] failed to parse embedded experiments.json ({error}); treating all experiments as disabled"
                );
                return ExperimentManifest {
                    known_ids: HashSet::new(),
                    default_enabled: HashSet::new(),
                };
            }
        };
        let mut known_ids = HashSet::new();
        let mut default_enabled = HashSet::new();
        for entry in parsed.experiments {
            known_ids.insert(entry.id.clone());
            if entry.default_enabled {
                default_enabled.insert(entry.id);
            }
        }
        ExperimentManifest {
            known_ids,
            default_enabled,
        }
    })
}

#[derive(Deserialize)]
struct RawManifest {
    #[allow(dead_code)]
    version: u32,
    experiments: Vec<RawExperiment>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawExperiment {
    id: String,
    #[serde(default)]
    default_enabled: bool,
}

/// User overrides pushed from the UI: experiment id → explicit enabled state.
///
/// Only experiments whose toggled state differs from the manifest default appear
/// here; empty for a user who never opened Settings → Experiments.
#[derive(Default)]
pub struct Experiments {
    overrides: Mutex<HashMap<String, bool>>,
}

impl Experiments {
    pub fn new() -> Self {
        Self::default()
    }

    /// `set_experiment_overrides`: replace the whole override map.
    pub fn set_overrides(&self, overrides: HashMap<String, bool>) {
        if let Ok(mut guard) = self.overrides.lock() {
            *guard = overrides;
        }
    }

    /// Whether `id` is effectively enabled: a user override wins, otherwise the
    /// manifest's `defaultEnabled`. Unknown ids are never enabled — a stale
    /// experiment id in a saved session must not unlock anything.
    pub fn is_enabled(&self, id: &str) -> bool {
        let manifest = manifest();
        if !manifest.known_ids.contains(id) {
            return false;
        }
        if let Ok(guard) = self.overrides.lock() {
            if let Some(&enabled) = guard.get(id) {
                return enabled;
            }
        }
        manifest.default_enabled.contains(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id_with_default(enabled: bool) -> String {
        let parsed: RawManifest =
            serde_json::from_str(EXPERIMENTS_MANIFEST_JSON).expect("embedded manifest parses");
        parsed
            .experiments
            .into_iter()
            .find(|entry| entry.default_enabled == enabled)
            .map(|entry| entry.id)
            .expect("the manifest declares both kinds of experiment")
    }

    #[test]
    fn the_embedded_manifest_parses() {
        assert!(!manifest().known_ids.is_empty());
        assert!(!manifest().default_enabled.is_empty());
    }

    #[test]
    fn an_unknown_id_is_never_enabled() {
        let experiments = Experiments::new();
        assert!(!experiments.is_enabled("definitely-not-an-experiment"));
        experiments.set_overrides(
            [("definitely-not-an-experiment".to_string(), true)]
                .into_iter()
                .collect(),
        );
        assert!(!experiments.is_enabled("definitely-not-an-experiment"));
    }

    #[test]
    fn the_manifest_default_applies_without_an_override() {
        let experiments = Experiments::new();
        let off = id_with_default(false);
        let on = id_with_default(true);
        assert!(!experiments.is_enabled(&off));
        assert!(experiments.is_enabled(&on));
    }

    #[test]
    fn an_override_wins_in_both_directions() {
        let experiments = Experiments::new();
        let off = id_with_default(false);
        let on = id_with_default(true);

        experiments.set_overrides(
            [(off.clone(), true), (on.clone(), false)]
                .into_iter()
                .collect(),
        );
        assert!(experiments.is_enabled(&off));
        assert!(!experiments.is_enabled(&on));

        // Replacing the map clears what it did not mention.
        experiments.set_overrides(HashMap::new());
        assert!(!experiments.is_enabled(&off));
        assert!(experiments.is_enabled(&on));
    }
}

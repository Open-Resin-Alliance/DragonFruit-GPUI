//! Scripted input for the shell: `--scenario <file.json>`.
//!
//! The pixels harness this port already has proves a control *renders*; it
//! cannot tell a live button from a dead one. This module drives the real
//! window through GPUI's own input pipeline
//! ([`Window::dispatch_event`](gpui::Window) with a [`PlatformInput`]), so every
//! click runs the same listeners a mouse does, then snapshots the state the
//! control was supposed to change and prints the delta.
//!
//! # Mode
//!
//! `DragonFruit::new` reads `--scenario <path>`; the shell still opens its
//! window (as `--smoke` does) and paints normally, and the driver below is
//! spawned from the first frame. The store is redirected to a temporary
//! directory in this mode, so a scenario never reads or writes the user's
//! `settings.json` / `session.json`.
//!
//! # Scenario file
//!
//! ```json
//! {
//!   "name": "app bar",
//!   "model": "../test-cube.stl",
//!   "steps": [
//!     { "kind": "click", "target": "appbar/settings" },
//!     { "kind": "expect", "state": { "show_settings": true } },
//!     { "kind": "click", "x": 640, "y": 400 },
//!     { "kind": "key", "key": "Escape" },
//!     { "kind": "wheel", "delta": 3 },
//!     { "kind": "wait", "ms": 900 }
//!   ]
//! }
//! ```
//!
//! `model` is optional: when the command line names no file, the app opens it,
//! resolved against the scenario file's directory. A scenario without a `model`
//! runs on an empty plate, which is all the app bar needs.
//!
//! A `click` needs either `target` (a name from [`resolve`]) or both `x` and
//! `y`; `button` is `left` (default), `right` or `middle`. A snapshot is taken
//! after every step and diffed against the previous one; `expect` checks that
//! same snapshot's fields — numbers with a relative tolerance, everything else
//! by equality.
//!
//! # What cannot be driven yet
//!
//! * **Text entry.** `gpui-ui`'s `text_field` takes its keystrokes through the
//!   platform input handler, not through `on_key_down`; a dispatch of
//!   `KeyDown` carries `key_char: None`, so nothing is inserted. Typing would
//!   need the window's input handler to be driven directly.
//! * **Drags.** This module only emits a press/release pair at one point. The
//!   gizmo and plate grab need a move *with* `pressed_button` between them,
//!   plus a hover pass so the handle is picked first; both are possible with
//!   [`PlatformInput::MouseMove`], but no scenario needs it yet, so it is not
//!   implemented rather than half-implemented.
//! * **File dialogs.** `Import mesh…` opens an OS dialog.
//!
//! # Targets
//!
//! Named targets map to the element ids the shell already uses
//! (`app-menu-button`, `settings-button`, `menu-new-scene`, rail tile ids,
//! `step/1`…) and are resolved by [`resolve`] into a window-space rectangle.
//! The rectangle is computed from the same constants the element is laid out
//! with (`app_bar.rs`, `rail.rs`, `gpui-core::theme::TOPBAR_HEIGHT`), because
//! GPUI exposes no hitbox query outside its test harness. A stale constant
//! therefore shows up as a failing `expect`, which is the point.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use gpui::{
    AsyncWindowContext, Context, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, PlatformInput, Point, ScrollDelta,
    ScrollWheelEvent, WeakEntity, Window, point, px,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{DragonFruit, Mode, SupportPanel};

// ── Scenario file ───────────────────────────────────────────────────────────

/// A parsed `--scenario` file.
#[derive(Debug, Deserialize)]
pub(crate) struct Scenario {
    /// Shown in the report's heading; defaults to the file name.
    #[serde(default)]
    name: Option<String>,
    /// The mesh the scenario needs, relative to the scenario file. Used when
    /// the command line names no file, so `dragonfruit-gpui --scenario
    /// scenarios/support-rail.json` works on its own.
    #[serde(default)]
    model: Option<String>,
    steps: Vec<Step>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Step {
    /// Pointer move, press and release at one point.
    Click {
        #[serde(default)]
        target: Option<String>,
        #[serde(default)]
        x: Option<f32>,
        #[serde(default)]
        y: Option<f32>,
        #[serde(default)]
        button: Button,
    },
    /// Pointer move without a button.
    Hover {
        #[serde(default)]
        target: Option<String>,
        #[serde(default)]
        x: Option<f32>,
        #[serde(default)]
        y: Option<f32>,
    },
    /// A pressed and released key, e.g. `Escape`.
    Key { key: String },
    /// A scroll wheel event; positive lines scroll up.
    Wheel {
        #[serde(default)]
        target: Option<String>,
        #[serde(default)]
        x: Option<f32>,
        #[serde(default)]
        y: Option<f32>,
        #[serde(default = "one_line")]
        delta: f32,
    },
    /// Sleep before taking the snapshot.
    Wait {
        #[serde(default = "default_wait")]
        ms: u64,
    },
    /// Compare the snapshot to these fields. Takes no input.
    Expect { state: Map<String, Value> },
}

fn one_line() -> f32 {
    1.0
}

fn default_wait() -> u64 {
    250
}

#[derive(Debug, Default, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Button {
    #[default]
    Left,
    Right,
    Middle,
}

impl Button {
    fn as_mouse(self) -> MouseButton {
        match self {
            Self::Left => MouseButton::Left,
            Self::Right => MouseButton::Right,
            Self::Middle => MouseButton::Middle,
        }
    }
}

impl Scenario {
    /// Read and parse a scenario file, naming the path in every error.
    pub(crate) fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
    }

    fn title(&self) -> String {
        self.name.clone().unwrap_or_else(|| "scenario".to_owned())
    }

    /// The model the scenario declared, resolved against the scenario file's
    /// own directory so a relative path in the JSON is not relative to the
    /// process's working directory.
    pub(crate) fn model_path(&self, scenario: &Path) -> Option<std::path::PathBuf> {
        let model = std::path::PathBuf::from(self.model.as_deref()?);
        if model.is_absolute() {
            return Some(model);
        }
        scenario.parent().map(|parent| parent.join(model))
    }
}

impl Step {
    fn describe(&self) -> String {
        match self {
            Self::Click {
                target,
                x,
                y,
                button,
            } => {
                let at = target
                    .clone()
                    .unwrap_or_else(|| format!("({}, {})", x.unwrap_or(0.0), y.unwrap_or(0.0)));
                match button {
                    Button::Left => format!("click {at}"),
                    Button::Right => format!("right-click {at}"),
                    Button::Middle => format!("middle-click {at}"),
                }
            }
            Self::Hover { target, x, y } => format!(
                "hover {}",
                target.clone().unwrap_or_else(|| format!(
                    "({}, {})",
                    x.unwrap_or(0.0),
                    y.unwrap_or(0.0)
                ))
            ),
            Self::Key { key } => format!("key {key}"),
            Self::Wheel {
                target,
                x,
                y,
                delta,
                ..
            } => format!(
                "wheel {delta} at {}",
                target.clone().unwrap_or_else(|| format!(
                    "({}, {})",
                    x.unwrap_or(0.0),
                    y.unwrap_or(0.0)
                ))
            ),
            Self::Wait { ms } => format!("wait {ms}ms"),
            Self::Expect { state } => {
                let keys: Vec<&str> = state.keys().map(String::as_str).collect();
                format!("expect {}", keys.join(", "))
            }
        }
    }
}

// ── Snapshot ────────────────────────────────────────────────────────────────

/// The parts of the shell a scenario can assert on: the workspace, the model's
/// placement, the camera, every overlay flag, and the panel ids the frame
/// actually mounted.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Snapshot {
    pub mode: String,
    pub has_mesh: bool,
    pub selected: bool,
    pub active_tool: usize,
    pub support_panel: String,
    pub translate: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub camera_yaw: f32,
    pub camera_pitch: f32,
    pub camera_distance: f32,
    pub camera_target: [f32; 3],
    pub camera_orthographic: bool,
    /// `Camera::home`'s pose: target `(0, 0, 110)`, distance `|(-220, -220,
    /// 260)|`. True once the home tween has settled.
    pub camera_at_home: bool,
    pub show_settings: bool,
    pub show_support_settings: bool,
    pub show_printer_profiles: bool,
    pub show_printer_picker: bool,
    pub show_app_menu: bool,
    pub show_view_menu: bool,
    pub show_printer_menu: bool,
    pub editor_menu: bool,
    pub toast: Option<String>,
    pub show_grid: bool,
    pub show_plate: bool,
    pub show_box: bool,
    pub overhang_view: bool,
    pub cross_section: bool,
    pub layer: f32,
    pub lower_layer: f32,
    pub num_layers: u32,
    /// The ids the frame passed to `panel_stack`, in mount order.
    pub panels: Vec<String>,
    pub panel_count: usize,
}

/// `Camera::home`'s target.
const HOME_TARGET: glam::Vec3 = glam::Vec3::new(0.0, 0.0, 110.0);

/// `Camera::home`'s distance, `|(-220, -220, 260)|`.
fn home_distance() -> f32 {
    glam::Vec3::new(-220.0, -220.0, 260.0).length()
}

fn support_panel_name(panel: SupportPanel) -> &'static str {
    match panel {
        SupportPanel::Islands => "islands",
        SupportPanel::Manual => "manual",
        SupportPanel::Auto => "auto",
        SupportPanel::Hollowing => "hollowing",
    }
}

impl DragonFruit {
    /// The state a scenario step is judged by.
    pub(crate) fn snapshot(&self) -> Snapshot {
        let camera = self.camera.borrow();
        let at_home = (camera.target - HOME_TARGET).length() < 0.5
            && (camera.distance - home_distance()).abs() < 0.5;
        let transform = self.transform();
        Snapshot {
            mode: self.mode.label().to_owned(),
            has_mesh: self.has_mesh(),
            selected: self.selected(),
            active_tool: self.active_tool,
            support_panel: support_panel_name(self.support_panel).to_owned(),
            translate: transform.translate.to_array(),
            rotation: transform.rotation,
            scale: transform.scale.to_array(),
            camera_yaw: camera.yaw,
            camera_pitch: camera.pitch,
            camera_distance: camera.distance,
            camera_target: camera.target.to_array(),
            camera_orthographic: camera.orthographic,
            camera_at_home: at_home,
            show_settings: self.show_settings,
            show_support_settings: self.show_support_settings,
            show_printer_profiles: self.show_printer_profiles,
            show_printer_picker: self.show_printer_picker,
            show_app_menu: self.show_app_menu,
            show_view_menu: self.show_view_menu,
            show_printer_menu: self.show_printer_menu,
            editor_menu: self.editor_menu.is_some(),
            toast: self.toast.as_ref().map(|toast| toast.text.clone()),
            show_grid: self.show_grid,
            show_plate: self.show_plate,
            show_box: self.show_box,
            overhang_view: self.overhang_view,
            cross_section: self.cross_section,
            layer: self.layer,
            lower_layer: self.lower_layer,
            num_layers: self.num_layers,
            panels: self
                .mounted_panels
                .iter()
                .map(|id| (*id).to_owned())
                .collect(),
            panel_count: self.mounted_panels.len(),
        }
    }
}

// ── Driver ──────────────────────────────────────────────────────────────────

/// Spawn the scenario driver from the first frame. The task waits for the
/// window to paint, then plays the steps one at a time.
pub(crate) fn start(scenario: Scenario, window: &mut Window, cx: &mut Context<DragonFruit>) {
    cx.spawn_in(window, async move |this, cx| {
        run(this, cx, scenario).await;
    })
    .detach();
}

async fn run(this: WeakEntity<DragonFruit>, cx: &mut AsyncWindowContext, scenario: Scenario) {
    // The driver is spawned from the first render; that frame's paint has
    // completed by the time this task is first polled, so its hitboxes are in
    // `rendered_frame` and the first click can land. The zero-count wait is a
    // yield that also gives the window a turn to draw the frame that spawned
    // this task.
    wait_for_frames(&this, cx, 0, 10_000).await;
    let width = this.read_with(cx, |app, _| app.window_width).unwrap_or(0.0);
    if width <= 0.0 {
        eprintln!("scenario: the window never rendered");
        std::process::exit(1);
    }

    let Ok(initial) = this.read_with(cx, |app, _| app.snapshot()) else {
        eprintln!("scenario: the window's root view is gone");
        std::process::exit(1);
    };
    let mut previous = initial;

    println!();
    println!("## {}", scenario.title());
    println!();
    println!("| # | step | state delta | expect |");
    println!("|---|------|-------------|--------|");

    let mut failures = 0usize;
    let mut steps = 0usize;

    for (index, step) in scenario.steps.iter().enumerate() {
        steps += 1;
        if let Step::Wait { ms } = step {
            // An explicit wait lets a tween (the camera home, the model
            // framing) run; the frames come from its own animation requests.
            sleep(cx, *ms).await;
        } else if let Err(error) = play(step, &this, cx) {
            eprintln!("scenario: step {index}: {error}");
            println!("| {index} | {} | — | FAIL: {error} |", step.describe());
            failures += 1;
            break;
        }
        // A step that changed the shell invalidated the window; the next frame
        // is the one whose hitboxes the following step's click tests against.
        // A step that changed nothing (a disabled target) times out harmlessly.
        if !matches!(step, Step::Expect { .. } | Step::Wait { .. }) {
            wait_for_frames(&this, cx, 1, 400).await;
        }

        let Ok(current) = this.read_with(cx, |app, _| app.snapshot()) else {
            eprintln!("scenario: the window's root view is gone");
            std::process::exit(1);
        };
        let delta = diff(&previous, &current);
        previous = current.clone();

        let verdict = match step {
            Step::Expect { state } => match check(state, &current) {
                Ok(()) => "ok".to_owned(),
                Err(message) => {
                    failures += 1;
                    // The expected/actual pair is in `message`; the whole
                    // snapshot is the context that explains it.
                    if let Ok(state) = serde_json::to_string(&current) {
                        eprintln!("scenario: step {index} state: {state}");
                    }
                    format!("FAIL: {message}")
                }
            },
            _ => String::new(),
        };
        println!(
            "| {index} | {} | {} | {verdict} |",
            step.describe(),
            if delta.is_empty() {
                "—".to_owned()
            } else {
                delta.join("; ")
            }
        );
    }

    println!();
    if failures == 0 {
        println!("scenario: {steps} steps, all expectations held");
    } else {
        println!("scenario: {steps} steps, {failures} failed");
    }
    let _ = std::io::stdout().flush();

    std::process::exit(if failures == 0 { 0 } else { 1 });
}

/// Dispatch one step's input. Returns the reason a step could not run (an
/// unknown target, a missing coordinate); `expect` is checked after the
/// snapshot.
fn play(
    step: &Step,
    this: &WeakEntity<DragonFruit>,
    cx: &mut AsyncWindowContext,
) -> Result<(), String> {
    match step {
        Step::Expect { .. } => Ok(()),
        Step::Wait { .. } => Ok(()),
        Step::Click {
            target,
            x,
            y,
            button,
        } => {
            let point = resolve_point(this, cx, target.as_deref(), *x, *y)?;
            dispatch_click(cx, button.as_mouse(), point);
            Ok(())
        }
        Step::Hover { target, x, y } => {
            let point = resolve_point(this, cx, target.as_deref(), *x, *y)?;
            dispatch_move(cx, point);
            Ok(())
        }
        Step::Key { key } => {
            dispatch_key(cx, key);
            Ok(())
        }
        Step::Wheel {
            target,
            x,
            y,
            delta,
        } => {
            let point = resolve_point(this, cx, target.as_deref(), *x, *y)?;
            dispatch_wheel(cx, point, *delta);
            Ok(())
        }
    }
}

/// A step's window-space point: an explicit coordinate, or the centre of a
/// named target in the current frame's state.
fn resolve_point(
    this: &WeakEntity<DragonFruit>,
    cx: &mut AsyncWindowContext,
    target: Option<&str>,
    x: Option<f32>,
    y: Option<f32>,
) -> Result<Point<Pixels>, String> {
    if let Some(name) = target {
        let rect = this
            .read_with(cx, |app, _| resolve(name, app))
            .map_err(|_| "the window's root view is gone".to_owned())?
            .ok_or_else(|| format!("unknown target '{name}'"))?;
        return Ok(point(px(rect.x + rect.w / 2.0), px(rect.y + rect.h / 2.0)));
    }
    match (x, y) {
        (Some(x), Some(y)) => Ok(point(px(x), px(y))),
        _ => Err("a step needs either `target` or both `x` and `y`".to_owned()),
    }
}

fn dispatch_move(cx: &mut AsyncWindowContext, position: Point<Pixels>) {
    let _ = cx.update(|window, app| {
        window.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent {
                position,
                ..Default::default()
            }),
            app,
        );
    });
}

fn dispatch_click(cx: &mut AsyncWindowContext, button: MouseButton, position: Point<Pixels>) {
    let _ = cx.update(|window, app| {
        // Move first: the click listeners test `hitbox.is_hovered`, which reads
        // the hit test `dispatch_event` rebuilds from the pointer position.
        window.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent {
                position,
                ..Default::default()
            }),
            app,
        );
        window.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                button,
                position,
                click_count: 1,
                ..Default::default()
            }),
            app,
        );
        window.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                button,
                position,
                click_count: 1,
                ..Default::default()
            }),
            app,
        );
    });
}

fn dispatch_key(cx: &mut AsyncWindowContext, key: &str) {
    let keystroke = Keystroke {
        modifiers: Modifiers::default(),
        key: key.to_owned(),
        key_char: None,
    };
    let _ = cx.update(|window, app| {
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: keystroke.clone(),
                is_held: false,
                prefer_character_input: false,
            }),
            app,
        );
        window.dispatch_event(PlatformInput::KeyUp(KeyUpEvent { keystroke }), app);
    });
}

fn dispatch_wheel(cx: &mut AsyncWindowContext, position: Point<Pixels>, delta: f32) {
    let _ = cx.update(|window, app| {
        window.dispatch_event(
            PlatformInput::ScrollWheel(ScrollWheelEvent {
                position,
                delta: ScrollDelta::Lines(point(0.0, delta)),
                ..Default::default()
            }),
            app,
        );
    });
}

async fn sleep(cx: &mut AsyncWindowContext, ms: u64) {
    if ms == 0 {
        return;
    }
    cx.background_executor()
        .timer(Duration::from_millis(ms))
        .await;
}

/// Wait until `count` more frames have been rendered, giving up after
/// `cap_ms`. A step that changes no state renders nothing, so the cap is not
/// an error.
async fn wait_for_frames(
    this: &WeakEntity<DragonFruit>,
    cx: &mut AsyncWindowContext,
    count: u64,
    cap_ms: u64,
) {
    let Ok(start) = this.read_with(cx, |app, _| app.frames) else {
        return;
    };
    let mut waited = 0;
    while waited < cap_ms {
        sleep(cx, 25).await;
        waited += 25;
        match this.read_with(cx, |app, _| app.frames) {
            Ok(now) if now >= start.saturating_add(count) => return,
            Ok(_) => {}
            Err(_) => return,
        }
    }
}

// ── Expectations and the report table ───────────────────────────────────────

fn check(state: &Map<String, Value>, snapshot: &Snapshot) -> Result<(), String> {
    let actual = serde_json::to_value(snapshot).map_err(|error| error.to_string())?;
    let mut failures = Vec::new();
    for (key, expected) in state {
        match actual.get(key) {
            None => failures.push(format!("`{key}` is not a snapshot field")),
            Some(value) => {
                if !json_matches(value, expected) {
                    failures.push(format!(
                        "`{key}` expected {} got {}",
                        display(expected),
                        display(value)
                    ));
                }
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

/// Equality with a relative tolerance on numbers, so a scenario can write
/// `405.46` for a computed distance.
fn json_matches(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Number(left), Value::Number(right)) => match (left.as_f64(), right.as_f64()) {
            (Some(left), Some(right)) => (left - right).abs() <= 1e-3 * left.abs().max(1.0),
            _ => actual == expected,
        },
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|(left, right)| json_matches(left, right))
        }
        _ => actual == expected,
    }
}

fn diff(previous: &Snapshot, current: &Snapshot) -> Vec<String> {
    let before = flatten(previous);
    let after = flatten(current);
    let mut changes = Vec::new();
    for (key, value) in &after {
        match before.get(key) {
            Some(old) if old == value => {}
            Some(old) => changes.push(format!("`{key}` {old} → {value}")),
            None => changes.push(format!("`{key}` → {value}")),
        }
    }
    changes
}

fn flatten(snapshot: &Snapshot) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if let Ok(Value::Object(fields)) = serde_json::to_value(snapshot) {
        for (key, value) in fields {
            out.insert(key, display(&value));
        }
    }
    out
}

fn display(value: &Value) -> String {
    match value {
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(display).collect::<Vec<_>>().join(", ")
        ),
        Value::Number(number) => match number.as_f64() {
            Some(number) if number.fract() == 0.0 && number.abs() < 1.0e9 => {
                format!("{number:.0}")
            }
            Some(number) => format!("{number:.3}"),
            None => number.to_string(),
        },
        Value::String(text) => text.clone(),
        Value::Bool(flag) => flag.to_string(),
        Value::Null => "null".to_owned(),
        other => other.to_string(),
    }
}

// ── Target registry ─────────────────────────────────────────────────────────
//
// GPUI keeps hitboxes private, so a name cannot be looked up at runtime. Each
// rectangle below is the arithmetic of the element that carries the same id in
// the tree, using the constants that element is laid out with. Every block
// names the file the constants come from; if one of those files changes the
// scenario fails, which is the early warning the old pixel harness lacked.

#[derive(Clone, Copy, Debug)]
pub(crate) struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

// `app_bar.rs`: the shell is `h = px(48.)`, `px_4`, `gap_12`, the two side
// groups are `flex_1 max_w_430` and the centre is what remains.
const BAR_HEIGHT: f32 = 48.0;
const BAR_PAD: f32 = 4.0;
const BAR_GAP: f32 = 12.0;
const SIDE_MAX: f32 = 430.0;

// `rail.rs`: a 54px column at `left-1.5` under the theme's
// `TOPBAR_HEIGHT` (52), 54px tiles, `gap-1.5`, `py-1.5`, and `mb-3` / `mt-3`
// group margins.
const RAIL_LEFT: f32 = 6.0;
const RAIL_WIDTH: f32 = 54.0;
const RAIL_TOP: f32 = 52.0;
const TILE: f32 = 54.0;
const RAIL_GAP: f32 = 6.0;
const RAIL_PAD_Y: f32 = 6.0;
const GROUP_GAP: f32 = 12.0;

// `app_bar.rs`'s steps row: `gap(px(8.))` between four `flex_1` buttons of
// `min_w 90` / `max_w 190`, inside the centre group's `px_8`.
const STEP_GAP: f32 = 8.0;
const STEP_MIN: f32 = 90.0;
const STEP_MAX: f32 = 190.0;
const STEP_PAD_X: f32 = 8.0;

// `gpui-ui::ui::context_menu_sized` (192px) placed at `(4, 49)` by
// `app_bar::app_menu`: 1px border + 6px padding, then the title row
// (`py-4`, 10px at 1.45 line height, `mb-1`) and 32px rows with `gap(2)`,
// 9px group rules and 8px row padding.
const MENU_LEFT: f32 = 4.0;
const MENU_TOP: f32 = 49.0;
const MENU_WIDTH: f32 = 192.0;
const MENU_BORDER: f32 = 1.0;
const MENU_PAD: f32 = 6.0;
const MENU_ROW_H: f32 = 32.0;
const MENU_GAP: f32 = 2.0;
const MENU_RULE_H: f32 = 9.0;
const MENU_TITLE_H: f32 = 22.5;
const MENU_TITLE_BOTTOM: f32 = 4.0;

/// The app menu's rows in `app_menu` order: id and whether a group rule opens
/// above it.
const APP_MENU_ROWS: [(&str, bool); 7] = [
    ("menu-new-scene", false),
    ("menu-save-scene", true),
    ("menu-save-scene-as", false),
    ("menu-open-scene", false),
    ("menu-import-mesh", true),
    ("menu-import-scene", false),
    ("menu-close-program", true),
];

/// Prepare's rail tiles after the Models toggle, in `PREPARE_TOOLS` order; the
/// flag is `Separated::Above`.
const PREPARE_RAIL: [(&str, bool); 8] = [
    ("select", false),
    ("transform", false),
    ("placeOnFace", false),
    ("mirror", false),
    ("duplicate", true),
    ("arrange", false),
    ("organicCut", true),
    ("smoothing", false),
];

/// Support's rail tiles after the Models toggle, in `entries_for` order; the
/// flag is `Separated::Above`.
const SUPPORT_RAIL: [(&str, bool); 6] = [
    ("islands", false),
    ("manual", true),
    ("auto", false),
    ("hollowing", true),
    ("interiorView", false),
    ("viewMode", true),
];

/// The bar's three columns: left edge, side-group width, centre width.
fn bar_columns(width: f32) -> (f32, f32, f32) {
    let content = (width - 2.0 * BAR_PAD).max(0.0);
    let share = ((content - 2.0 * BAR_GAP) / 3.0).max(0.0);
    let side = share.min(SIDE_MAX);
    let centre = (content - 2.0 * BAR_GAP - 2.0 * side).max(0.0);
    (BAR_PAD, side, centre)
}

fn step_rect(index: usize, width: f32) -> Rect {
    let (left, side, centre) = bar_columns(width);
    let steps_x = left + side + BAR_GAP;
    let inner = (centre - 2.0 * STEP_PAD_X).max(0.0);
    let per = ((inner - 3.0 * STEP_GAP) / 4.0).clamp(STEP_MIN, STEP_MAX);
    let total = 4.0 * per + 3.0 * STEP_GAP;
    let row_x = steps_x + STEP_PAD_X + (inner - total) / 2.0;
    Rect {
        x: row_x + index as f32 * (per + STEP_GAP),
        y: 8.0,
        w: per,
        h: 32.0,
    }
}

/// The right group's controls, anchored from the window's right edge so the
/// view-mode label's width cannot shift them: `pr_3`, three 32px caption
/// buttons with 4px gaps and a 4px left margin, an 8px gap, then the 34px
/// toolbar buttons.
fn settings_rect(width: f32) -> Rect {
    let caption = 3.0 * 32.0 + 2.0 * 4.0;
    let right = width - BAR_PAD - 3.0 - caption - 4.0 - 8.0;
    Rect {
        x: right - 34.0,
        y: 6.0,
        w: 34.0,
        h: 36.0,
    }
}

fn overhang_rect(width: f32) -> Rect {
    let settings = settings_rect(width);
    Rect {
        x: settings.x - 8.0 - 34.0,
        ..settings
    }
}

fn view_type_rect(width: f32) -> Rect {
    // `!w-9` (36) when the label is hidden; when shown the box is wider and
    // still contains this point.
    let overhang = overhang_rect(width);
    Rect {
        x: overhang.x - 8.0 - 36.0,
        y: 6.0,
        w: 36.0,
        h: 36.0,
    }
}

fn app_menu_rect(width: f32) -> Rect {
    let (left, ..) = bar_columns(width);
    Rect {
        x: left,
        y: 4.0,
        w: 40.0,
        h: 40.0,
    }
}

fn printer_rect(width: f32) -> Rect {
    // 40px logo button, `gap-2.5` (10), the `w-px` divider with `mx-0.5` (5),
    // `gap-2.5` (10). The button is at most 300px wide; its left padding and
    // thumbnail put this point inside it whatever the label measures.
    let (left, ..) = bar_columns(width);
    Rect {
        x: left + 40.0 + 10.0 + 5.0 + 10.0,
        y: 4.0,
        w: 44.0,
        h: 40.0,
    }
}

fn app_menu_row(name: &str) -> Option<Rect> {
    let content_x = MENU_LEFT + MENU_BORDER + MENU_PAD;
    let content_w = MENU_WIDTH - 2.0 * (MENU_BORDER + MENU_PAD);
    let mut y = MENU_TOP + MENU_BORDER + MENU_PAD + MENU_TITLE_H + MENU_TITLE_BOTTOM;
    for (index, (id, starts_group)) in APP_MENU_ROWS.iter().enumerate() {
        if index > 0 {
            y += MENU_GAP;
            if *starts_group {
                y += MENU_RULE_H + MENU_GAP;
            }
        }
        if *id == name {
            return Some(Rect {
                x: content_x,
                y,
                w: content_w,
                h: MENU_ROW_H,
            });
        }
        y += MENU_ROW_H;
    }
    None
}

fn rail_entries(mode: Mode) -> Vec<(&'static str, bool)> {
    // The Models toggle is column-only and opens with a group rule below it.
    let mut entries: Vec<(&'static str, bool)> = vec![("models", false)];
    match mode {
        Mode::Prepare => entries.extend(PREPARE_RAIL.iter().map(|(id, above)| (*id, *above))),
        Mode::Support => entries.extend(SUPPORT_RAIL.iter().map(|(id, above)| (*id, *above))),
        Mode::Export | Mode::Printing => return Vec::new(),
    }
    entries
}

fn rail_rect(name: &str, mode: Mode) -> Option<Rect> {
    let entries = rail_entries(mode);
    if entries.is_empty() {
        return None;
    }
    let mut y = RAIL_TOP + RAIL_PAD_Y;
    for (index, (id, above)) in entries.into_iter().enumerate() {
        if index > 0 {
            y += RAIL_GAP;
            if above {
                y += GROUP_GAP;
            }
        }
        if id == name {
            return Some(Rect {
                x: RAIL_LEFT,
                y,
                w: RAIL_WIDTH,
                h: TILE,
            });
        }
        y += TILE;
        // The Models toggle carries `mb-3`.
        if id == "models" {
            y += GROUP_GAP;
        }
    }
    None
}

/// Resolve a target name for the app's *current* state.
///
/// Accepted forms: the element id (`settings-button`, `menu-new-scene`,
/// `islands`), a namespaced path (`appbar/settings`, `menu/new-scene`,
/// `rail/islands`, `step/support`), or `canvas` for the scene area.
pub(crate) fn resolve(name: &str, app: &DragonFruit) -> Option<Rect> {
    let width = if app.window_width > 0.0 {
        app.window_width
    } else {
        // `main.rs` opens a 1280x800 window; fall back to it before the first
        // frame has recorded a width.
        1280.0
    };

    match name {
        "app-menu-button" | "appbar/menu" => Some(app_menu_rect(width)),
        "printer-menu-button" | "appbar/printer" => Some(printer_rect(width)),
        "settings-button" | "appbar/settings" => Some(settings_rect(width)),
        "overhang-toggle" | "appbar/overhang" => Some(overhang_rect(width)),
        "view-type-trigger" | "appbar/view" => Some(view_type_rect(width)),
        "step/prepare" | "step/1" => Some(step_rect(0, width)),
        "step/support" | "step/2" => Some(step_rect(1, width)),
        "step/export" | "step/3" => Some(step_rect(2, width)),
        "step/printing" | "step/4" => Some(step_rect(3, width)),
        "canvas" | "canvas/center" => Some(Rect {
            x: 0.0,
            y: BAR_HEIGHT,
            w: width,
            h: 800.0 - BAR_HEIGHT,
        }),
        _ => {
            // `menu/new-scene` names `menu-new-scene`; the table stores the
            // element id, so rebuild it rather than stripping the prefix.
            if let Some(row) = name.strip_prefix("menu/") {
                return app_menu_row(&format!("menu-{row}"));
            }
            if let Some(tile) = name.strip_prefix("rail/") {
                return rail_rect(tile, app.mode);
            }
            if APP_MENU_ROWS.iter().any(|(id, _)| *id == name) {
                return app_menu_row(name);
            }
            rail_rect(name, app.mode)
        }
    }
}

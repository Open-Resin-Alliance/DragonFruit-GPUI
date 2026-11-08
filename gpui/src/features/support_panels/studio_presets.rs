//! The Support Studio's preset collection — `src/supports/Settings/presets.ts`
//! (`PresetCollection`) and the `PresetSelector` / preset strip that reads it.
//!
//! `gpui-store` models the document ([`gpui_store::SupportPresets`]); this
//! module is its owner in the app. It opens the store once, keeps the collection
//! in memory for the panel to render, and writes `support-presets.json` back on
//! every change, so the selected preset and the values it holds survive a
//! restart instead of resetting to the panel's own defaults.
//!
//! # Why this module owns a `Store`
//!
//! The panel is handed a `WeakEntity<DragonFruit>` and nothing else, and
//! `DragonFruit::store` is private to `app.rs`. [`gpui_store::Store`] is safe to
//! open twice — it resolves the data directory and writes atomically — and the
//! document has no other owner in the port, so the collection keeps its own
//! handle rather than routing each click through a shell hook.
//!
//! # PORT DEVIATIONS
//!
//! 1. **The block a preset shows here.** A studio preset owns the geometry tree
//!    (`tip` / `shaft` / `roots` / `baseFlare` / `joint` / `grid`), and
//!    `setActivePreset` *preserves* the live `autoSupport` block on selection —
//!    "auto-support density is owned by the Auto Support panel, never by the
//!    trunk-preset dropdown". The port's Manual panel is the auto-support knob
//!    editor, so this module shows and edits the record's
//!    `settings.autoSupport` block, which every studio record carries. Every
//!    other key of a stored record round-trips untouched.
//! 2. **Built-ins are code with one block.** `DETAIL_PRESET` /
//!    `STRUCTURE_PRESET` / `ANCHOR_PRESET` carry their identity and a complete
//!    `autoSupport` block (`createDefaultAutoSupportSettings()` with the
//!    preset's pinned area and sizing band). Their geometry tree is not
//!    reproduced: no ported UI reads it, and a record the React build wrote
//!    keeps its own tree, because [`commit_values`] merges into
//!    `settings.autoSupport` and writes the rest of `settings` back as-is.
//! 3. **Committed, not live.** The source persists the live settings on every
//!    edit and the preset on `Save`; the port has one document, so an edit
//!    stages in the draft and `Save` commits it into the active custom record.
//!    A selection, a duplicate, a delete and a rename are written immediately.
//! 4. **No drag, no pinning, no bulk delete.** `PresetSelector`'s drag reorder,
//!    the four quick-access slots and the bulk-delete dialog are not ported;
//!    `pinnedSlot` is carried through a stored record but never edited.
//! 5. **No rename.** The source opens a name dialog with a text field; this
//!    slice has no text entry, so the panel keeps `Rename` disabled (the
//!    dialog's own note records the same gap).

use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use gpui_core::support_state::SupportSettingsValues;
use gpui_store::{Store, StoreFile, SupportPreset, SupportPresets, SUPPORT_PRESETS_SCHEMA_VERSION};
use serde_json::{Map, Value};

/// `presets.ts`'s `defaults.activePresetId`.
pub const DEFAULT_PRESET_ID: &str = "structure";

/// The factory records `presets.ts` re-derives on every launch. They are code,
/// never written to the document (the store crate's own rule).
const BUILT_IN_IDS: [&str; 3] = ["detail", "structure", "anchor"];

/// The document's in-memory state.
struct StudioPresets {
    /// `None` when the data directory cannot be opened; the collection then
    /// lives for the session and nothing is persisted.
    store: Option<Store>,
    /// Rail order: the three factory records, then the user's own.
    presets: Vec<SupportPreset>,
    /// `activePresetId`.
    active: String,
    /// What the panel shows: seeded from the active record and staged until
    /// `Save`.
    draft: SupportSettingsValues,
    /// The active record's stored values; `Reset` restores them.
    baseline: SupportSettingsValues,
    /// The strip's dropdown.
    menu_open: bool,
}

impl StudioPresets {
    fn load() -> Self {
        let store = Store::open().ok();
        let stored = match store.as_ref() {
            Some(store) => store.load_support_presets(),
            None => SupportPresets::default(),
        };

        let mut presets = built_in_presets();
        for preset in stored.presets {
            // The factory ids are code: a stored copy of one is ignored, as
            // `loadPresetsFromStorage` does while merging the defaults.
            if preset.id.is_empty() || is_built_in(&preset.id) {
                continue;
            }
            if presets.iter().any(|existing| existing.id == preset.id) {
                continue;
            }
            presets.push(preset);
        }

        let active = stored
            .active_preset_id
            .filter(|id| presets.iter().any(|preset| &preset.id == id))
            .unwrap_or_else(|| DEFAULT_PRESET_ID.to_string());
        let values = values_of(&presets, &active);

        Self {
            store,
            presets,
            active,
            draft: values,
            baseline: values,
            menu_open: false,
        }
    }

    fn active_record(&self) -> Option<&SupportPreset> {
        self.presets.iter().find(|preset| preset.id == self.active)
    }

    fn active_values(&self) -> SupportSettingsValues {
        values_of(&self.presets, &self.active)
    }
}

static PRESETS: LazyLock<Mutex<StudioPresets>> = LazyLock::new(|| Mutex::new(StudioPresets::load()));

fn state() -> &'static Mutex<StudioPresets> {
    &PRESETS
}

fn with<R>(f: impl FnOnce(&mut StudioPresets) -> R) -> R {
    match state().lock() {
        Ok(mut guard) => f(&mut guard),
        Err(poisoned) => f(&mut poisoned.into_inner()),
    }
}

/// `isBuiltIn` in `presets.ts`: the three ids the code owns.
pub fn is_built_in(id: &str) -> bool {
    BUILT_IN_IDS.contains(&id)
}

// ── Reads ───────────────────────────────────────────────────────────────────

/// The strip's rows — `(id, name, is built-in)` — in rail order.
///
/// Only the identity, so a panel redraw does not clone every record's
/// `settings` tree.
pub fn preset_rows() -> Vec<(String, String, bool)> {
    with(|state| {
        state
            .presets
            .iter()
            .map(|preset| {
                (
                    preset.id.clone(),
                    preset.name.clone(),
                    is_built_in(&preset.id),
                )
            })
            .collect()
    })
}

/// `getActivePresetId`.
pub fn active_id() -> String {
    with(|state| state.active.clone())
}

/// The active record's name, or `None` when the collection lost it.
pub fn active_name() -> Option<String> {
    with(|state| state.active_record().map(|preset| preset.name.clone()))
}

/// `getActivePreset`. `petal`-free clone of the record the panel reads.
pub fn active_preset() -> Option<SupportPreset> {
    with(|state| state.active_record().cloned())
}

/// The values the panel's controls show.
pub fn values() -> SupportSettingsValues {
    with(|state| state.draft)
}

/// Whether the draft differs from the active record (`isPresetDirty`).
pub fn is_modified() -> bool {
    with(|state| !same_values(&state.draft, &state.baseline))
}

pub fn menu_open() -> bool {
    with(|state| state.menu_open)
}

// ── Edits ───────────────────────────────────────────────────────────────────

/// Stage an edit. `Save` commits it into the active custom record.
pub fn set_value(f: impl FnOnce(&mut SupportSettingsValues)) {
    with(|state| f(&mut state.draft));
}

pub fn toggle_menu() {
    with(|state| state.menu_open = !state.menu_open);
}

pub fn close_menu() {
    with(|state| state.menu_open = false);
}

/// `setActivePreset`: the record becomes the selection and its block becomes
/// the panel's values. Written at once — this is the fact that must survive a
/// restart.
pub fn select(id: &str) -> bool {
    with(|state| {
        if !state.presets.iter().any(|preset| preset.id == id) {
            return false;
        }
        state.active = id.to_string();
        let values = state.active_values();
        state.draft = values;
        state.baseline = values;
        state.menu_open = false;
        save(state);
        true
    })
}

/// `Revert` / the footer's Reset: the active record's stored values again.
pub fn reset() -> SupportSettingsValues {
    with(|state| {
        let values = state.active_values();
        state.draft = values;
        state.baseline = values;
        values
    })
}

/// The footer's Save: writes the draft into the active record.
///
/// A built-in is refused, matching the source's disabled Save — Duplicate
/// first, which is the way out of a locked profile.
pub fn save_active() -> Option<SupportSettingsValues> {
    with(|state| {
        let draft = state.draft;
        let active = state.active.clone();
        if is_built_in(&active) {
            return None;
        }
        let preset = state
            .presets
            .iter_mut()
            .find(|preset| preset.id == active)?;
        preset.updated_at = Some(now_millis());
        commit_values(preset, &draft);
        state.baseline = draft;
        save(state);
        Some(draft)
    })
}

/// `duplicatePreset`: a custom copy of the active record, made the selection.
/// This is the source's way out of a locked built-in, and it persists.
pub fn duplicate_active() -> Option<String> {
    with(|state| {
        let source = state.active_record()?.clone();
        let id = next_custom_id(&state.presets);
        let name = unique_name(&state.presets, &format!("{} copy", source.name), None);
        let now = now_millis();

        let mut copy = source;
        copy.id = id.clone();
        copy.name = name;
        copy.description = Some("User custom preset".to_string());
        copy.icon = Some("\u{1f464}".to_string());
        copy.is_built_in = false;
        copy.hotkey = None;
        copy.pinned_slot = None;
        copy.created_at = Some(now);
        copy.updated_at = Some(now);

        state.presets.push(copy);
        state.active = id.clone();
        let values = state.active_values();
        state.draft = values;
        state.baseline = values;
        state.menu_open = false;
        save(state);
        Some(id)
    })
}

/// `deletePreset`: a built-in is refused; the selection falls back to the first
/// record, as the source's does.
pub fn delete_active() -> bool {
    with(|state| {
        let id = state.active.clone();
        if is_built_in(&id) {
            return false;
        }
        let before = state.presets.len();
        state.presets.retain(|preset| preset.id != id);
        if state.presets.len() == before {
            return false;
        }
        state.active = state
            .presets
            .first()
            .map(|preset| preset.id.clone())
            .unwrap_or_else(|| DEFAULT_PRESET_ID.to_string());
        let values = state.active_values();
        state.draft = values;
        state.baseline = values;
        save(state);
        true
    })
}

/// `renamePreset`: the name is made unique across the collection.
pub fn rename_active(name: &str) -> bool {
    with(|state| {
        let active = state.active.clone();
        if is_built_in(&active) {
            return false;
        }
        let desired = match name.trim() {
            "" => "Custom Preset".to_string(),
            trimmed => trimmed.to_string(),
        };
        let unique = unique_name(&state.presets, &desired, Some(active.as_str()));
        let Some(preset) = state
            .presets
            .iter_mut()
            .find(|preset| preset.id == active)
        else {
            return false;
        };
        preset.name = unique;
        preset.updated_at = Some(now_millis());
        save(state);
        true
    })
}

// ── The document ────────────────────────────────────────────────────────────

/// Write `support-presets.json`. The factory records are filtered out: they are
/// re-derived on the next launch.
fn save(state: &StudioPresets) {
    let Some(store) = state.store.as_ref() else {
        return;
    };
    if store.is_read_only(StoreFile::SupportPresets) {
        return;
    }
    let document = SupportPresets {
        schema_version: SUPPORT_PRESETS_SCHEMA_VERSION,
        active_preset_id: Some(state.active.clone()),
        presets: state
            .presets
            .iter()
            .filter(|preset| !is_built_in(&preset.id))
            .cloned()
            .collect(),
    };
    let _ = store.save_support_presets(&document);
}

// ── The `autoSupport` block ─────────────────────────────────────────────────

fn values_of(presets: &[SupportPreset], id: &str) -> SupportSettingsValues {
    presets
        .iter()
        .find(|preset| preset.id == id)
        .map(|preset| values_from_settings(&preset.settings))
        .unwrap_or_default()
}

/// The panel's nine knobs, read out of a record's `settings.autoSupport`.
///
/// A key the record does not carry falls back to the constraint default the
/// whole app shares (`SupportSettingsValues::default()`, i.e.
/// `AUTO_SUPPORT_CONSTRAINTS`), never to a value of this module's own.
fn values_from_settings(settings: &Value) -> SupportSettingsValues {
    let defaults = SupportSettingsValues::default();
    let Some(block) = settings.get("autoSupport").and_then(Value::as_object) else {
        return defaults;
    };
    SupportSettingsValues {
        area_per_support_mm2: number(
            block,
            "areaPerSupportMm2",
            defaults.area_per_support_mm2,
        ),
        size_scale: number(block, "sizeScale", defaults.size_scale),
        grid_area_threshold_mm2: number(
            block,
            "gridAreaThresholdMm2",
            defaults.grid_area_threshold_mm2,
        ),
        flat_density_boost: number(block, "flatDensityBoost", defaults.flat_density_boost),
        slope_relax_factor: number(block, "slopeRelaxFactor", defaults.slope_relax_factor),
        suction_area_exponent: number(
            block,
            "suctionAreaExponent",
            defaults.suction_area_exponent,
        ),
        self_support_angle_deg: number(
            block,
            "overhangSelfSupportAngleDeg",
            defaults.self_support_angle_deg,
        ),
        stabilization_enabled: flag(
            block,
            "stabilizationEnabled",
            defaults.stabilization_enabled,
        ),
        minima_reinforcement_enabled: flag(
            block,
            "minimaReinforcementEnabled",
            defaults.minima_reinforcement_enabled,
        ),
    }
}

/// Merge the nine knobs into a record's `settings.autoSupport`, leaving every
/// other key of `settings` untouched so a record written by the React build
/// keeps its geometry tree.
fn commit_values(preset: &mut SupportPreset, values: &SupportSettingsValues) {
    if !preset.settings.is_object() {
        preset.settings = Value::Object(Map::new());
    }
    let Value::Object(root) = &mut preset.settings else {
        return;
    };
    let block = root
        .entry("autoSupport".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !block.is_object() {
        *block = Value::Object(Map::new());
    }
    let Value::Object(block) = block else {
        return;
    };
    block.insert(
        "areaPerSupportMm2".to_string(),
        Value::from(values.area_per_support_mm2 as f64),
    );
    block.insert("sizeScale".to_string(), Value::from(values.size_scale as f64));
    block.insert(
        "gridAreaThresholdMm2".to_string(),
        Value::from(values.grid_area_threshold_mm2 as f64),
    );
    block.insert(
        "flatDensityBoost".to_string(),
        Value::from(values.flat_density_boost as f64),
    );
    block.insert(
        "slopeRelaxFactor".to_string(),
        Value::from(values.slope_relax_factor as f64),
    );
    block.insert(
        "suctionAreaExponent".to_string(),
        Value::from(values.suction_area_exponent as f64),
    );
    block.insert(
        "overhangSelfSupportAngleDeg".to_string(),
        Value::from(values.self_support_angle_deg as f64),
    );
    block.insert(
        "stabilizationEnabled".to_string(),
        Value::Bool(values.stabilization_enabled),
    );
    block.insert(
        "minimaReinforcementEnabled".to_string(),
        Value::Bool(values.minima_reinforcement_enabled),
    );
}

fn number(block: &Map<String, Value>, key: &str, fallback: f32) -> f32 {
    block
        .get(key)
        .and_then(Value::as_f64)
        .map(|value| value as f32)
        .unwrap_or(fallback)
}

fn flag(block: &Map<String, Value>, key: &str, fallback: bool) -> bool {
    block.get(key).and_then(Value::as_bool).unwrap_or(fallback)
}

/// `same_values`, by bit pattern: the draft and the baseline come from the same
/// document, so an untouched field is identical, not merely close.
fn same_values(a: &SupportSettingsValues, b: &SupportSettingsValues) -> bool {
    a.area_per_support_mm2.to_bits() == b.area_per_support_mm2.to_bits()
        && a.size_scale.to_bits() == b.size_scale.to_bits()
        && a.grid_area_threshold_mm2.to_bits() == b.grid_area_threshold_mm2.to_bits()
        && a.flat_density_boost.to_bits() == b.flat_density_boost.to_bits()
        && a.slope_relax_factor.to_bits() == b.slope_relax_factor.to_bits()
        && a.suction_area_exponent.to_bits() == b.suction_area_exponent.to_bits()
        && a.self_support_angle_deg.to_bits() == b.self_support_angle_deg.to_bits()
        && a.stabilization_enabled == b.stabilization_enabled
        && a.minima_reinforcement_enabled == b.minima_reinforcement_enabled
}

// ── The factory records ─────────────────────────────────────────────────────

/// `DETAIL_PRESET`, `STRUCTURE_PRESET` and `ANCHOR_PRESET`.
fn built_in_presets() -> Vec<SupportPreset> {
    vec![
        SupportPreset {
            id: "detail".to_string(),
            name: "Detail".to_string(),
            description: Some("Fine supports for delicate features".to_string()),
            hotkey: Some("1".to_string()),
            icon: Some("\u{1f52c}".to_string()),
            is_built_in: false,
            pinned_slot: Some(1),
            settings: auto_support_block(16.0, "detail"),
            created_at: None,
            updated_at: None,
        },
        SupportPreset {
            id: "structure".to_string(),
            name: "Structure".to_string(),
            description: Some("Balanced supports for general use".to_string()),
            hotkey: Some("2".to_string()),
            icon: Some("\u{1f3d7}\u{fe0f}".to_string()),
            is_built_in: false,
            pinned_slot: Some(2),
            settings: auto_support_block(
                SupportSettingsValues::default().area_per_support_mm2,
                "structure",
            ),
            created_at: None,
            updated_at: None,
        },
        SupportPreset {
            id: "anchor".to_string(),
            name: "Anchor".to_string(),
            description: Some("Heavy supports for large overhangs".to_string()),
            hotkey: Some("3".to_string()),
            icon: Some("\u{2693}".to_string()),
            is_built_in: false,
            pinned_slot: Some(3),
            settings: auto_support_block(5.0, "anchor"),
            created_at: None,
            updated_at: None,
        },
    ]
}

/// A record's `settings` for the factory bands: `createDefaultAutoSupportSettings()`
/// with the preset's pinned area and sizing band, exactly as `presets.ts` pins
/// them. The geometry tree is deliberately absent (see deviation 2).
fn auto_support_block(area_per_support_mm2: f32, sizing_preset: &str) -> Value {
    let mut auto = Map::new();
    let mut put = |key: &str, value: Value| {
        auto.insert(key.to_string(), value);
    };
    put("enabled", Value::Bool(true));
    put("minIslandAreaMm2", Value::from(0.02));
    put("tipInfluenceRadiusMm", Value::from(0.5));
    put("prioritizeIntersection", Value::Bool(false));
    put("maxAttachmentsPerTrunk", Value::from(12));
    put(
        "areaPerSupportMm2",
        Value::from(area_per_support_mm2 as f64),
    );
    put("gridAreaThresholdMm2", Value::from(25));
    put("overhangSelfSupportAngleDeg", Value::from(45));
    put("sizeScale", Value::from(1));
    put("flatDensityBoost", Value::from(0.7));
    put("slopeRelaxFactor", Value::from(1.3));
    put("suctionAreaExponent", Value::from(0.15));
    put("coverageTargetPercent", Value::from(95));
    put("sizingPreset", Value::from(sizing_preset));
    put("leafFanRadiusMm", Value::from(8));
    put("leafFanMaxAngleDeg", Value::from(45));
    put("tipContactMarginScale", Value::from(0.9));
    put("memberHostShaftRatio", Value::from(0.7));
    put("modelScaleEnabled", Value::Bool(true));
    put("modelSizeFactorCap", Value::from(1.45));
    put("modelLoadFactorCap", Value::from(1.3));
    put("heightFactorCap", Value::from(1.35));
    put("debugSupportOriginColors", Value::Bool(false));
    put("debugSkipAutoBracing", Value::Bool(false));
    put("stabilizationEnabled", Value::Bool(true));
    put("minimaReinforcementEnabled", Value::Bool(true));
    // Ends the closure's borrow so the map can be moved into the tree.
    drop(put);

    let mut root = Map::new();
    root.insert("autoSupport".to_string(), Value::Object(auto));
    Value::Object(root)
}

// ── Names and ids ───────────────────────────────────────────────────────────

/// `ensureUniqueName`: case-insensitive, `" (2)"` onwards.
fn unique_name(presets: &[SupportPreset], desired: &str, exclude: Option<&str>) -> String {
    let base = match desired.trim() {
        "" => "Custom Preset",
        trimmed => trimmed,
    };
    let taken = |candidate: &str| {
        let lower = candidate.to_lowercase();
        presets.iter().any(|preset| {
            Some(preset.id.as_str()) != exclude && preset.name.to_lowercase() == lower
        })
    };

    let mut name = base.to_string();
    let mut counter = 1;
    while taken(&name) {
        counter += 1;
        name = format!("{base} ({counter})");
    }
    name
}

/// `makeCustomPresetId`: `custom-<base36 millis>` and a counter for the
/// (impossible in practice) same-millisecond duplicate. No RNG in this crate.
fn next_custom_id(presets: &[SupportPreset]) -> String {
    let base = format!("custom-{}", base36(now_millis().max(0) as u64));
    let mut candidate = base.clone();
    let mut counter = 1;
    while presets.iter().any(|preset| preset.id == candidate) {
        counter += 1;
        candidate = format!("{base}-{counter}");
    }
    candidate
}

fn base36(mut value: u64) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if value == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while value > 0 {
        out.push(DIGITS[(value % 36) as usize]);
        value /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_else(|_| "0".to_string())
}

/// `Date.now()` — epoch milliseconds, the unit `createdAt` / `updatedAt` use.
fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0)
}

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
//! 1. **The live auto-support block is not the preset's.** `setActivePreset`
//!    *preserves* the live `autoSupport` block on selection — "Trunk presets
//!    are sizing/geometry profiles; auto-support density is owned by the Auto
//!    Support panel ... never by the trunk-preset dropdown" (`presets.ts:683-693`)
//!    — and `savePreset` restores the record's own block rather than writing
//!    the live one into it (`presets.ts:723-731`). This module therefore holds
//!    the panel's knobs in its own live block ([`StudioPresets::draft`]),
//!    seeded once from the Auto Support module's live settings at load
//!    ([`crate::features::auto_support_settings::live_settings`]) and never re-seeded by a
//!    selection nor written back by `Save`.
//! 2. **Factory records are ordinary editable records, not a lock.** All three
//!    ship `isBuiltIn: false` (`presets.ts:146-153,210-217,239-246`) and
//!    `presets.ts:779` states "All presets are now editable (no built-in
//!    lock)". Their ids only identify the code-owned records the document never
//!    stores; the panel's lock is [`active_locked`], the record's own flag.
//!    Their complete `autoSupport` block is carried through the record, but it
//!    is never the live settings; their geometry tree is not reproduced, because
//!    no ported UI reads it and every other key of a stored record round-trips
//!    untouched.
//! 3. **Save acknowledges the live block.** The source persists the live
//!    settings on every edit and the preset on `Save`; the port's `Save` marks
//!    the live block as acknowledged (clearing the dirty star) and leaves the
//!    record's `autoSupport` alone, as `savePreset` does. A selection, a
//!    duplicate, a delete and a rename are written to the document immediately.
//! 4. **No drag, no pinning, no bulk delete.** `PresetSelector`'s drag reorder,
//!    the four quick-access slots and the bulk-delete dialog are not ported;
//!    `pinnedSlot` is carried through a stored record but never edited.
//! 5. **No rename.** The source opens a name dialog with a text field; this
//!    slice has no text entry, so `Rename` is gated the way the source gates it
//!    (`selectedPreset.isBuiltIn`, `PresetSelector.tsx:543-544`) and is inert
//!    once enabled.

use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use gpui_core::support_state::SupportSettingsValues;
use gpui_store::{Store, StoreFile, SupportPreset, SupportPresets, SUPPORT_PRESETS_SCHEMA_VERSION};
use serde_json::{Map, Value};

/// `presets.ts`'s `defaults.activePresetId`.
pub const DEFAULT_PRESET_ID: &str = "structure";

/// The factory records `presets.ts` re-derives on every launch. They are code,
/// never written to the document (the store crate's own rule). This is
/// *identity*, not a lock: all three ship `isBuiltIn: false`.
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
    /// The live auto-support block the panel edits (`getSettings().autoSupport`).
    /// Seeded once from [`crate::features::auto_support_settings::live_settings`]; a selection
    /// never re-seeds it. `Save` acknowledges the block, `Reset` restores the
    /// acknowledgement.
    draft: SupportSettingsValues,
    /// The block as last acknowledged by `Save`; `Reset` restores it.
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
        let values = values_from_live(crate::features::auto_support_settings::live_settings());

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

/// The three ids `presets.ts` owns as code (`isBuiltIn`'s *name* is misleading
/// here): the document never stores them, so they are filtered out on both
/// load and save.
///
/// This is identity, **not** a lock. All three factory records ship
/// `isBuiltIn: false` (`presets.ts:146-153,210-217,239-246`); the panel's lock
/// is [`active_locked`].
pub fn is_built_in(id: &str) -> bool {
    BUILT_IN_IDS.contains(&id)
}

// ── Reads ───────────────────────────────────────────────────────────────────

/// The strip's rows — `(id, name, is factory record)` — in rail order.
///
/// Only the identity, so a panel redraw does not clone every record's
/// `settings` tree. The flag marks the three records `presets.ts` owns as code
/// (the sizing bands the Auto Support dialog lists); it does not imply a lock.
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

/// The panel's lock: the *record's own* `isBuiltIn` flag, never its id.
///
/// `presets.ts` ships every factory record with `isBuiltIn: false`
/// (`:146-153`, `:210-217`, `:239-246`) and states "All presets are now
/// editable (no built-in lock)" (`:779`); `AutoSupportPanel.tsx:125` derives
/// its `presetLocked` from the same flag.
pub fn active_locked() -> bool {
    with(|state| {
        state
            .active_record()
            .map_or(false, |preset| preset.is_built_in)
    })
}

/// The active record's name, or `None` when the collection lost it.
pub fn active_name() -> Option<String> {
    with(|state| state.active_record().map(|preset| preset.name.clone()))
}

/// `getActivePreset`. `petal`-free clone of the record the panel reads.
pub fn active_preset() -> Option<SupportPreset> {
    with(|state| state.active_record().cloned())
}

/// The live auto-support block the panel's controls show.
pub fn values() -> SupportSettingsValues {
    with(|state| state.draft)
}

/// Whether the live block differs from the last `Save` (`isPresetDirty`).
pub fn is_modified() -> bool {
    with(|state| !same_values(&state.draft, &state.baseline))
}

pub fn menu_open() -> bool {
    with(|state| state.menu_open)
}

// ── Edits ───────────────────────────────────────────────────────────────────

/// Stage an edit on the live auto-support block. `Save` acknowledges it; the
/// selected record's own `settings.autoSupport` is left untouched.
pub fn set_value(f: impl FnOnce(&mut SupportSettingsValues)) {
    with(|state| f(&mut state.draft));
}

pub fn toggle_menu() {
    with(|state| state.menu_open = !state.menu_open);
}

pub fn close_menu() {
    with(|state| state.menu_open = false);
}

/// `setActivePreset`: the record becomes the selection, written at once — this
/// is the fact that must survive a restart.
///
/// The live auto-support block is deliberately **not** re-seeded. The source
/// keeps `autoSupport: { ...current.autoSupport }` (`presets.ts:683-693`), so
/// switching profile must not change a single knob of the live block (nor the
/// angle the app classifies with); the preset's own stored block is never read
/// into the live settings.
pub fn select(id: &str) -> bool {
    with(|state| {
        if !state.presets.iter().any(|preset| preset.id == id) {
            return false;
        }
        state.active = id.to_string();
        state.menu_open = false;
        save(state);
        true
    })
}

/// `Revert` / the footer's Reset: the live block as `Save` last left it.
pub fn reset() -> SupportSettingsValues {
    with(|state| {
        state.draft = state.baseline;
        state.baseline
    })
}

/// The footer's Save: acknowledges the live block.
///
/// The draft is **not** merged into the active record: `savePreset` restores
/// the record's own `autoSupport` block (`presets.ts:723-731`), so a preset is
/// never polluted with the live one. A record whose own `isBuiltIn` is set
/// refuses the save, as the source footer's Save does
/// (`AutoSupportPresets.tsx:630-633`); every factory record in `presets.ts` is
/// `isBuiltIn: false`, so a fresh profile saves freely.
pub fn save_active() -> Option<SupportSettingsValues> {
    with(|state| {
        if state
            .active_record()
            .map_or(true, |preset| preset.is_built_in)
        {
            return None;
        }
        let draft = state.draft;
        state.baseline = draft;
        Some(draft)
    })
}

/// `duplicatePreset`: a custom copy of the active record, made the selection.
/// This is the source's way out of a locked record, it persists, and it leaves
/// the live auto-support block where it was (`setActivePreset` preserves it).
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
        state.menu_open = false;
        save(state);
        Some(id)
    })
}

/// `deletePreset`: a record whose own `isBuiltIn` is set is refused; the
/// selection falls back to the first record, as the source's does. The live
/// auto-support block is untouched.
pub fn delete_active() -> bool {
    with(|state| {
        let id = state.active.clone();
        if state
            .active_record()
            .map_or(true, |preset| preset.is_built_in)
        {
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
        save(state);
        true
    })
}

/// `renamePreset`: the name is made unique across the collection. Gated on the
/// record's own `isBuiltIn`, as `PresetSelector.tsx:543-544` gates inline
/// rename; the live auto-support block is untouched.
pub fn rename_active(name: &str) -> bool {
    with(|state| {
        let active = state.active.clone();
        if state
            .active_record()
            .map_or(true, |preset| preset.is_built_in)
        {
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

/// The panel's nine knobs, read from the live block the Auto Support module owns
/// (`getSettings().autoSupport`).
///
/// This is the one moment the panel reads the block; a later selection never
/// re-seeds it (`setActivePreset` preserves it, `presets.ts:683-693`), and it is
/// never read from a preset record — a studio preset's `settings.autoSupport`
/// is not the live settings.
fn values_from_live(
    live: crate::features::auto_support_settings::AutoSupportSettings,
) -> SupportSettingsValues {
    SupportSettingsValues {
        area_per_support_mm2: live.area_per_support_mm2,
        size_scale: live.size_scale,
        grid_area_threshold_mm2: live.grid_area_threshold_mm2,
        flat_density_boost: live.flat_density_boost,
        slope_relax_factor: live.slope_relax_factor,
        suction_area_exponent: live.suction_area_exponent,
        self_support_angle_deg: live.overhang_self_support_angle_deg,
        stabilization_enabled: live.stabilization_enabled,
        minima_reinforcement_enabled: live.minima_reinforcement_enabled,
    }
}

/// `same_values`, by bit pattern: the draft and the baseline come from the same
/// block, so an untouched field is identical, not merely close.
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

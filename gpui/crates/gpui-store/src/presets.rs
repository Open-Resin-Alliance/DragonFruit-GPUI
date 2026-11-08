//! `support-presets.json` — the user's support presets.
//!
//! Ported from `src/supports/Settings/presets.ts`, whose
//! `PRESET_STORAGE_KEY` (`'support-presets-v1'`) holds a `PresetCollection`
//! built from `SupportPreset` records (`src/supports/Settings/types.ts`).
//!
//! Only user presets are stored: the three built-ins (`detail`, `structure`,
//! `anchor`) are code in `presets.ts` and are re-derived on every launch. A
//! preset's `settings` is kept as raw JSON because the geometry tree
//! (`SupportSettings`) is owned by the supports crate, which this storage crate
//! must not depend on.

use serde::{Deserialize, Serialize};

/// Version this crate writes.
pub const SUPPORT_PRESETS_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportPresets {
    #[serde(default)]
    pub schema_version: u32,
    /// `PresetCollection.activePresetId`; `None` means "keep the built-in
    /// `'structure'` selection".
    #[serde(default)]
    pub active_preset_id: Option<String>,
    #[serde(default)]
    pub presets: Vec<SupportPreset>,
}

impl Default for SupportPresets {
    fn default() -> Self {
        Self {
            schema_version: SUPPORT_PRESETS_SCHEMA_VERSION,
            active_preset_id: None,
            presets: Vec::new(),
        }
    }
}

/// `SupportPreset` minus the code-owned geometry defaults.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportPreset {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub hotkey: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    /// Always `false` for records this file owns; kept so a record copied out
    /// of the React store round-trips unchanged.
    #[serde(default)]
    pub is_built_in: bool,
    /// Quick-access slot 1–4, or `None` if unpinned.
    #[serde(default)]
    pub pinned_slot: Option<u8>,
    /// The `SupportSettings` tree, validated by the supports crate that owns it.
    #[serde(default)]
    pub settings: serde_json::Value,
    #[serde(default)]
    pub created_at: Option<i64>,
    #[serde(default)]
    pub updated_at: Option<i64>,
}

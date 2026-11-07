//! Process state owned by `DragonFruit` and shared by every feature crate.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui::{App, Context, FocusHandle, MouseButton};

use crate::camera::Camera;
use crate::empty_state::{EmptySceneState, ModelStatsCardState};
use crate::panel_layout::PanelLayout;
use crate::stl;
use crate::support_state::{
    HolePunchState, HollowingState, IslandListState, IslandsPanelState, SupportSettingsState,
};
use crate::viewport::Viewport;

/// `MESH_SHADER_TYPES` order.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MeshShaderType {
    #[default]
    SoftClay,
    FlatUnlit,
    Matcap,
    NormalDebug,
    Wireframe,
    OpaqueWireMesh,
    Xray,
    OverhangHeatmap,
}

impl MeshShaderType {
    /// The stored value (`MESH_SHADER_TYPES`).
    pub fn value(self) -> &'static str {
        match self {
            Self::SoftClay => "soft_clay",
            Self::FlatUnlit => "flat_unlit",
            Self::Matcap => "matcap",
            Self::NormalDebug => "normal_debug",
            Self::Wireframe => "wireframe",
            Self::OpaqueWireMesh => "opaque_wire_mesh",
            Self::Xray => "xray",
            Self::OverhangHeatmap => "overhang_heatmap",
        }
    }

    /// `MESH_SHADER_OPTIONS[..].label`, or the raw value when the type is not
    /// one of the dropdown's options (`ViewTypeDropdown`'s `option ? label :
    /// value` fallback).
    pub fn label(self) -> &'static str {
        match self {
            Self::SoftClay => "Standard",
            Self::OverhangHeatmap => "Overhangs",
            Self::OpaqueWireMesh => "Mesh",
            Self::Wireframe => "Wireframe",
            Self::Xray => "X-Ray",
            Self::NormalDebug => "Normals",
            Self::FlatUnlit | Self::Matcap => self.value(),
        }
    }

    /// `getViewTypeIcon` from `ViewTypeDropdown.tsx`.
    pub fn icon(self) -> &'static str {
        match self {
            Self::SoftClay => "paintbrush",
            Self::NormalDebug => "scan",
            Self::Wireframe => "grid-3x3",
            Self::OverhangHeatmap => "alert-diamond-custom",
            Self::OpaqueWireMesh => "layers",
            Self::Xray => "eye",
            // `default:` — `flat_unlit` and `matcap` fall through.
            Self::FlatUnlit | Self::Matcap => "app-window",
        }
    }
}

/// `MESH_SHADER_OPTIONS`, in the order the dropdown lists them.

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastTone {
    Neutral,
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone)]
pub struct PrintingPreviewState {
    /// `printingPreviewTotalLayers` — `activeSlice.totalLayers`, `0` when the
    /// plate has not been sliced.
    pub total_layers: u32,
    /// `printingSelectedLayer` — the slider's upper thumb.
    pub selected_layer: f32,
    /// `crossSectionEnabled` — colours the slider and is what its double-click
    /// toggles.
    pub cross_section: bool,
    /// `printingPreviewPngUrlForDisplay` / `selectedPrintingLayerPreviewUrl`,
    /// resolved to a local file. `None` renders the empty state when the plate
    /// also has no layers.
    pub preview_image: Option<PathBuf>,
    /// `printingPreviewTargetResolution`'s `viewportWidth` / `viewportHeight`.
    /// Takes precedence over the printer profile, as at the source.
    pub viewport_mm: Option<(f32, f32)>,
    /// `activePrinterProfile.buildVolumeMm`'s `(width, depth)`. `None` is the
    /// source's `?? 143` / `?? 89` fallback.
    pub build_volume_mm: Option<(f32, f32)>,
}

impl Default for PrintingPreviewState {
    fn default() -> Self {
        Self {
            total_layers: 0,
            selected_layer: 1.0,
            cross_section: true,
            preview_image: None,
            viewport_mm: None,
            build_volume_mm: None,
        }
    }
}

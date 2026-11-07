//! Port of `src/components/settings/SettingsModal.tsx` and every tab body it
//! mounts: General, Camera, Workspace, Multi-Plate, Mesh, Slicing
//! (`PerformanceSettingsTab`), 3D Mouse, UI & Theme, Hotkeys, Plugins,
//! Experiments, Scene Autosave, Backups (`LocalBackupsSettingsTab`), UVTools,
//! Logging, Updates (`UpdatesSettingsTab` + `UpdateCheckerSection`) and About.
//!
//! Every size, colour and label traces to a class string or inline style in the
//! React source; Tailwind radii are the v4 scale (`rounded-sm` 4px, `-md` 6px,
//! `-lg` 8px, `-xl` 12px, `-2xl` 16px).
//!
//! PORT DEVIATIONS (each one forced by the GPUI slice; nothing is silently
//! simplified):
//! - The draft values live in one process-wide [`SettingsState`] because
//!   `DragonFruit` owns only `settings_tab`/`show_settings`. `Apply` and
//!   `Cancel` close the modal; persistence belongs to the parent.
//! - `SelectDropdown` is the real component now: the trigger opens a menu, the
//!   chosen row writes the draft and closes it. GPUI has no portal, so the menu
//!   renders as an absolute child under the trigger (6px below, full trigger
//!   width, 320px cap) and therefore scrolls with the panel; a click outside
//!   does not dismiss it — selecting a row or re-clicking the trigger does.
//! - `NumberInput` / `ScrollableNumberField` show the value and the source's
//!   steppers; typing (and wheel stepping) needs a text-input entity this slice
//!   does not have, so the box is not editable.
//! - Range inputs render as the `.ui-range` track with the accent fill and a
//!   knob at the current value; dragging is not wired in this slice.
//! - The shader preview canvas, `react-colorful` picker and colour-swatch
//!   inputs render as their containers filled with the current colour.
//! - Theme import/export, custom theme profiles and the nested confirm dialogs
//!   are not ported; the built-in preset row is what the source shows by
//!   default. The Slicing tab's temp-file cleanup buttons render but are not
//!   wired to the native slicer bridge.
//! - The active sidebar tab's 1px inset ring (`boxShadow`) has no GPUI
//!   equivalent and is omitted; its border and fill are kept.
//! - `MODEL_EDGE_OVERLAY_ENABLED` is `false` in the source, so the "Higher
//!   Contrast Model Edges" section is absent, exactly as in the app.
//! - No Tauri bridge exists in this slice. The Workspace tab therefore never
//!   sees an active printer profile (`isBuildVolumeManagedByPrinter` false), the
//!   3D Mouse tab never sees navlib active, and the bridge-backed sections of
//!   Scene Autosave, Backups, UVTools, Logging, Updates and Plugins render the
//!   source's own empty/never states with inert actions instead of inventing
//!   data.
//! - Hotkey rows show the source defaults; the recording overlay renders, but
//!   capturing a key needs the hotkey store's global dispatch, so the binding
//!   only changes once that store exists.
//! - Sponsors: remote avatars are not fetched, so every sponsor renders the
//!   source's initials monogram; the 34s marquee has no GPUI equivalent and the
//!   row is static. The tier ring is a conic gradient at the source and renders
//!   as the tier's solid colour.
//! - About reads the GPUI build's version/channel (there is no
//!   `NEXT_PUBLIC_*`), and the build-info chip cannot reach a clipboard, so it
//!   always shows the un-copied `Version …` state. The branding images
//!   (`text_logo.svg`, the ORA logo, `simple_icon.svg`) are not in the GPUI
//!   asset set: the wordmark and the ORA mark render as text and the trailing
//!   app icon is omitted.
//! - The Logging tab's `Copy path` tile cannot reach a clipboard either, so it
//!   is inert; the log path itself is the source's own non-desktop placeholder
//!   and the viewer shows its "desktop build" line.
//! - The Backups delete/restore confirmations can only open from a history row
//!   and the slice lists none; their copy interpolates the snapshot id where
//!   the source formats it as a locale timestamp (no date formatter here).
//! - Icon slugs the generated set lacks are mapped to the nearest available
//!   glyph: `Layers3`→`layers`, `Filter`→`funnel`, `UploadCloud`→`cloud-upload`,
//!   `AlertTriangle`→`triangle-alert`, `CheckCircle2`→`circle-check`,
//!   `RefreshCw`→`refresh-cw`, `RefreshCcw`→`refresh-ccw`.
//! - Backdrop blur, CSS transitions, text/box shadows and `z-index` stacking
//!   have no GPUI equivalent; overlays are painted after the card instead.
//!   GPUI gradients take two stops, so the source's three-or-more-stop
//!   gradients (the About badge, the credits divider) keep their first and last.
//!
//! The modal shell, the [`SettingsTab`] enum and the restore-defaults copy live
//! in this module; each tab body lives in the submodule named after it
//! (`general`, `camera`, `mesh`, `slicing`, `ui_theme`, `workspace`,
//! `multi_plate`, `spacemouse`, `hotkeys`, `plugins`, `experiments`, `autosave`,
//! `backups`, `uvtools`, `logging`, `updates`, `about`), with the enum/state
//! types in `state`, the source defaults in `defaults`, the theme colour tables
//! in `theme_colors` and the shared control recipes in `widgets`.

#![allow(dead_code)]

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{LazyLock, Mutex};

use gpui::{
    App, Div, FontWeight, IntoElement, MouseButton, SharedString, Stateful, WeakEntity, div,
    linear_color_stop, linear_gradient, prelude::*, px, relative, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_core::theme::*;
use gpui_ui::ui::{
    IconButtonSize, IconButtonVariant, IconTone, SelectOption, icon_button, icon_button_toned,
    select_dropdown, spinner,
};
use crate::DragonFruit;

pub mod about;
pub mod autosave;
pub mod backups;
pub mod camera;
pub mod defaults;
pub mod experiments;
pub mod general;
pub mod hotkeys;
pub mod logging;
pub mod mesh;
pub mod multi_plate;
pub mod plugins;
pub mod rows;
pub mod slicing;
pub mod spacemouse;
pub mod state;
pub mod theme_colors;
pub mod ui_theme;
pub mod updates;
pub mod uvtools;
pub mod widgets;
pub mod workspace;

pub(crate) use about::*;
pub(crate) use autosave::*;
pub(crate) use backups::*;
pub(crate) use camera::*;
pub(crate) use defaults::*;
pub(crate) use experiments::*;
pub(crate) use general::*;
pub(crate) use hotkeys::*;
pub(crate) use logging::*;
pub(crate) use mesh::*;
pub(crate) use multi_plate::*;
pub(crate) use plugins::*;
pub(crate) use rows::*;
pub(crate) use slicing::*;
pub(crate) use spacemouse::*;
pub(crate) use state::*;
pub(crate) use theme_colors::*;
pub(crate) use ui_theme::*;
pub(crate) use updates::*;
pub(crate) use uvtools::*;
pub(crate) use widgets::*;
pub(crate) use workspace::*;

/// Sidebar order: `sidebarTopTabs` then `sidebarBottomTabs`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Camera,
    Workspace,
    MultiPlate,
    Mesh,
    Slicing,
    Mouse3d,
    Ui,
    Hotkeys,
    Plugins,
    Experiments,
    SceneAutosave,
    Backups,
    UvTools,
    Logging,
    Updates,
    About,
}

struct TabMeta {
    key: &'static str,
    label: &'static str,
    description: &'static str,
    icon: &'static str,
    /// `tone: 'secondary'` in `tabMeta`; primary tabs use the accent.
    primary: bool,
}

impl SettingsTab {
    pub const ALL: [SettingsTab; 17] = [
        Self::General,
        Self::Camera,
        Self::Workspace,
        Self::MultiPlate,
        Self::Mesh,
        Self::Slicing,
        Self::Mouse3d,
        Self::Ui,
        Self::Hotkeys,
        Self::Plugins,
        Self::Experiments,
        Self::SceneAutosave,
        Self::Backups,
        Self::UvTools,
        Self::Logging,
        Self::Updates,
        Self::About,
    ];
    const TOP: [SettingsTab; 9] = [
        Self::General,
        Self::Camera,
        Self::Workspace,
        Self::MultiPlate,
        Self::Mesh,
        Self::Slicing,
        Self::Mouse3d,
        Self::Ui,
        Self::Hotkeys,
    ];
    const BOTTOM: [SettingsTab; 8] = [
        Self::Plugins,
        Self::Experiments,
        Self::SceneAutosave,
        Self::Backups,
        Self::UvTools,
        Self::Logging,
        Self::Updates,
        Self::About,
    ];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|tab| *tab == self).unwrap_or(0)
    }

    pub fn from_index(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or(Self::General)
    }

    fn meta(self) -> TabMeta {
        match self {
            Self::General => TabMeta {
                key: "general",
                label: "General",
                description: "Workspace behavior and panel layout",
                icon: "settings-2",
                primary: true,
            },
            Self::Camera => TabMeta {
                key: "camera",
                label: "Camera",
                description: "Projection and navigation behavior",
                icon: "camera",
                primary: true,
            },
            Self::Workspace => TabMeta {
                key: "workspaces",
                label: "Workspace",
                description: "Build volume bounds, origin, and 3D view resolution",
                icon: "monitor-cog",
                primary: true,
            },
            Self::MultiPlate => TabMeta {
                key: "multi-plate",
                label: "Multi-Plate",
                description: "Plate behaviour across several beds",
                icon: "square-stack",
                primary: true,
            },
            Self::Mesh => TabMeta {
                key: "mesh",
                label: "Mesh",
                description: "Shader, rendering options, and selection behavior",
                icon: "grid-3x3",
                primary: true,
            },
            Self::Slicing => TabMeta {
                key: "slicing",
                label: "Slicing",
                description: "PNG compression and engine metadata",
                icon: "monitor-cog",
                primary: true,
            },
            Self::Mouse3d => TabMeta {
                key: "spacemouse",
                label: "3D Mouse",
                description: "3D mouse navigation controls",
                icon: "gamepad-2",
                primary: true,
            },
            Self::Ui => TabMeta {
                key: "ui",
                label: "UI & Theme",
                description: "Theme and custom UI token customization",
                icon: "palette",
                primary: true,
            },
            Self::Hotkeys => TabMeta {
                key: "hotkeys",
                label: "Hotkeys",
                description: "Keyboard bindings and presets",
                icon: "keyboard",
                primary: true,
            },
            Self::Plugins => TabMeta {
                key: "plugins",
                label: "Plugins",
                description: "Load vendor profile plugins",
                icon: "plug",
                primary: false,
            },
            Self::Experiments => TabMeta {
                key: "experiments",
                label: "Experiments",
                description: "Early-access and experimental features",
                icon: "flask-conical",
                primary: false,
            },
            Self::SceneAutosave => TabMeta {
                key: "scene-autosave",
                label: "Scene Autosave",
                description: "Autosave and crash recovery behavior",
                icon: "hard-drive",
                primary: false,
            },
            Self::Backups => TabMeta {
                key: "backups",
                label: "Backups",
                description: "Local on-disk backup snapshots",
                icon: "archive-restore",
                primary: false,
            },
            Self::UvTools => TabMeta {
                key: "uvtools",
                label: "UVTools",
                description: "Send sliced files to UVTools for analysis",
                icon: "external-link",
                primary: false,
            },
            Self::Logging => TabMeta {
                key: "logging",
                label: "Logging",
                description: "Log file location and verbosity",
                icon: "scroll-text",
                primary: false,
            },
            Self::Updates => TabMeta {
                key: "updates",
                label: "Updates",
                description: "Check for new versions and manage channels",
                icon: "cloud-download",
                primary: false,
            },
            Self::About => TabMeta {
                key: "about",
                label: "About",
                description: "Version info and project details",
                icon: "info",
                primary: false,
            },
        }
    }
}

fn close(entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    entity
        .update(cx, |this, cx| {
            this.show_settings = false;
            cx.notify();
        })
        .ok();
}

/// `SettingsModal` for the app bar's settings button.
///
/// `open` mirrors `isOpen` (the parent also gates with `.when`); `tab` is the
/// parent-owned `settings_tab` index into [`SettingsTab::ALL`]. The close
/// button and the backdrop write `show_settings = false` on `entity`; Escape is
/// already handled by the parent's root key listener.
pub fn settings_modal(
    open: bool,
    tab: usize,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    if !open {
        return div();
    }
    {
        use std::sync::atomic::{AtomicBool, Ordering};
        static LOGGED: AtomicBool = AtomicBool::new(false);
        if !LOGGED.swap(true, Ordering::Relaxed) {
            gpui_core::log::log("settings: rendering open");
        }
    }

    // Snapshot the draft: holding the lock across `content` deadlocks because
    // the select fields lock it again to read `open_select`.
    let snapshot = state()
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default();
    let tab = SettingsTab::from_index(tab);

    let (body, overlay) = content(tab, &snapshot, entity.clone());

    let backdrop_entity = entity.clone();
    let card = div()
        .w_full()
        .max_w(px(CARD_MAX_WIDTH))
        .flex()
        .flex_col()
        .rounded(px(16.)) // rounded-2xl
        .border_1()
        .border_color(rgb(BORDER_STRONG))
        .bg(rgb(SURFACE_0))
        .overflow_hidden()
        // Keep backdrop clicks from closing the modal when they land on the card.
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(header(entity.clone()))
        .child(
            div()
                .flex_1()
                .min_h(px(0.))
                .flex()
                .child(sidebar(tab, entity.clone()))
                .child(body),
        )
        .child(footer(entity.clone()));

    let mut layer = div()
        .absolute()
        .top(px(0.))
        .left(px(0.))
        .size_full()
        .p(px(BACKDROP_PADDING))
        .flex()
        .items_stretch()
        .justify_center()
        .bg(rgba(0x00000099)) // bg-black/60
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            close(&backdrop_entity, cx);
        })
        .child(card);

    // The source portals every dialog to `<body>`; here they paint after the
    // card, inside the same backdrop.
    if let Some(overlay) = overlay {
        layer = layer.child(overlay);
    }
    if snapshot.show_restore_confirm {
        layer = layer.child(restore_defaults_dialog(entity));
    }
    layer
}

// ── Shell ────────────────────────────────────────────────────────────────

fn header(entity: WeakEntity<DragonFruit>) -> Div {
    let close_entity = entity;
    div()
        .flex()
        .items_center()
        .justify_between()
        .px(px(16.))
        .py(px(12.))
        .border_b_1()
        .border_color(rgb(BORDER_SUBTLE))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(36.))
                        .h(px(36.))
                        .rounded(px(8.)) // rounded-lg
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .bg(linear_gradient(
                            135.,
                            linear_color_stop(rgb(mix(ACCENT, SURFACE_1, 0.16)), 0.),
                            linear_color_stop(rgb(mix(ACCENT_SECONDARY, SURFACE_1, 0.10)), 1.),
                        ))
                        .child(icon("settings-2", 18., ACCENT)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(16.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Settings"),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(TEXT_MUTED))
                                .child("Customize DragonFruit behavior, visuals, and controls."),
                        ),
                ),
        )
        .child(
            icon_button(
                "settings-close",
                "x",
                IconButtonVariant::Solid,
                IconButtonSize::Sm,
            )
            .on_click(move |_, _, cx| close(&close_entity, cx)),
        )
}

fn sidebar(active: SettingsTab, entity: WeakEntity<DragonFruit>) -> Div {
    let mut top = div().flex().flex_col().gap(px(4.));
    for tab in SettingsTab::TOP {
        top = top.child(rail_tab(tab, tab == active, entity.clone()));
    }
    let mut bottom = div().flex().flex_col().gap(px(4.)).mt_auto().pt(px(12.));
    for tab in SettingsTab::BOTTOM {
        bottom = bottom.child(rail_tab(tab, tab == active, entity.clone()));
    }

    div()
        .w(px(SIDEBAR_WIDTH))
        .flex_shrink_0()
        .p(px(10.))
        .border_r_1()
        .border_color(rgb(BORDER_SUBTLE))
        // linear-gradient(180deg, surface-1 94%, accent-secondary 4% over surface-1)
        .bg(linear_gradient(
            180.,
            linear_color_stop(rgb(mix(SURFACE_1, SURFACE_0, 0.94)), 0.),
            linear_color_stop(rgb(mix(ACCENT_SECONDARY, SURFACE_1, 0.04)), 1.),
        ))
        .child(
            div()
                .id("settings-rail")
                .size_full()
                .min_h(px(0.))
                .flex()
                .flex_col()
                .overflow_y_scroll()
                .child(top)
                .child(bottom),
        )
}

fn rail_tab(tab: SettingsTab, active: bool, entity: WeakEntity<DragonFruit>) -> Stateful<Div> {
    let meta = tab.meta();
    let tone = if meta.primary { ACCENT } else { ACCENT_SECONDARY };
    let (border, background) = if active {
        (
            mix(tone, BORDER_SUBTLE, 0.65),
            mix(tone, SURFACE_0, 0.16),
        )
    } else {
        (BORDER_SUBTLE, SURFACE_1)
    };
    let icon_color = if active { tone } else { TEXT_MUTED };
    let index = tab.index();

    div()
        .id(format!("settings-rail-{}", meta.key))
        .w_full()
        .rounded(px(4.)) // rounded-sm
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .px(px(10.))
        .py(px(6.))
        .flex()
        .items_center()
        .gap(px(10.))
        .cursor_pointer()
        .on_click(move |_, _, cx| {
            entity
                .update(cx, |this, cx| {
                    this.settings_tab = index;
                    cx.notify();
                })
                .ok();
        })
        .child(icon(meta.icon, 20., icon_color))
        .child(
            div()
                .min_w(px(0.))
                .flex_1()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(relative(1.2))
                        .text_color(rgb(TEXT_STRONG))
                        .child(meta.label),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(relative(1.2))
                        .text_color(rgb(TEXT_MUTED))
                        .truncate()
                        .child(meta.description),
                ),
        )
}

fn content(
    tab: SettingsTab,
    state: &SettingsState,
    entity: WeakEntity<DragonFruit>,
) -> (Stateful<Div>, Option<Div>) {
    let body_entity = entity.clone();
    let body = match tab {
        SettingsTab::General => general_tab(state, body_entity),
        SettingsTab::Camera => camera_tab(state, body_entity),
        SettingsTab::Workspace => workspace_tab(state, body_entity),
        SettingsTab::MultiPlate => multi_plate_tab(state, body_entity),
        SettingsTab::Mesh => mesh_tab(state, body_entity),
        SettingsTab::Slicing => slicing_tab(state, body_entity),
        SettingsTab::Mouse3d => spacemouse_tab(state, body_entity),
        SettingsTab::Ui => ui_tab(state, body_entity),
        SettingsTab::Hotkeys => hotkeys_tab(state, body_entity),
        SettingsTab::Plugins => plugins_tab(state, body_entity),
        SettingsTab::Experiments => experiments_tab(state, body_entity),
        SettingsTab::SceneAutosave => autosave_tab(state, body_entity),
        SettingsTab::Backups => backups_tab(state, body_entity),
        SettingsTab::UvTools => uvtools_tab(state, body_entity),
        SettingsTab::Logging => logging_tab(state, body_entity),
        SettingsTab::Updates => updates_tab(state, body_entity),
        SettingsTab::About => about_tab(state, body_entity),
    };

    // About, Hotkeys and Updates use `usesInternalTabScrollLayout`: the tab owns
    // its own scroller and the content column is a fixed-height flex column.
    let internal_scroll = matches!(
        tab,
        SettingsTab::About | SettingsTab::Hotkeys | SettingsTab::Updates
    );

    let mut column = div()
        .id("settings-content")
        .flex_1()
        .min_w(px(0.))
        .min_h(px(0.))
        .p(px(16.));
    column = if internal_scroll {
        column.flex().flex_col()
    } else {
        column.overflow_y_scroll()
    };

    let body = body;
    let overlay = match tab {
        SettingsTab::Hotkeys => hotkeys_overlay(state, entity.clone()),
        SettingsTab::Plugins => plugins_overlay(state, entity.clone()),
        SettingsTab::Experiments => experiments_overlay(state, entity.clone()),
        SettingsTab::Backups => backups_overlay(state, entity.clone()),
        SettingsTab::Updates => updates_overlay(state, entity.clone()),
        _ => None,
    };

    (column.child(body), overlay)
}

fn footer(entity: WeakEntity<DragonFruit>) -> Div {
    let restore_entity = entity.clone();
    let cancel_entity = entity.clone();
    let apply_entity = entity;

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.))
        .px(px(12.))
        .py(px(12.))
        .border_t_1()
        .border_color(rgb(BORDER_SUBTLE))
        // color-mix(in srgb, surface-1, transparent 10%) over surface-0
        .bg(rgb(mix(SURFACE_1, SURFACE_0, 0.9)))
        .child(action_button(
            "settings-restore",
            "Restore Defaults",
            Some("rotate-ccw"),
            14.0,
            mix(ACCENT_SECONDARY, SURFACE_1, 0.08),
            mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.58),
            ACCENT_SECONDARY,
            mix(ACCENT_SECONDARY, SURFACE_1, 0.24),
            move |cx| {
                // `handleRestoreDefaults` opens the confirmation first.
                edit_and_notify(&restore_entity, cx, |draft| {
                    draft.show_restore_confirm = true;
                });
            },
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(action_button(
                    "settings-cancel",
                    "Cancel",
                    None,
                    16.0,
                    SURFACE_1,
                    BORDER_SUBTLE,
                    TEXT_MUTED,
                    SURFACE_2,
                    move |cx| close(&cancel_entity, cx),
                ))
                .child(action_button(
                    "settings-apply",
                    "Apply",
                    Some("check"),
                    16.0,
                    mix(ACCENT, SURFACE_0, 0.16),
                    mix(ACCENT, 0xffffff, 0.9),
                    ACCENT_CONTRAST,
                    mix(ACCENT, SURFACE_0, 0.28),
                    move |cx| close(&apply_entity, cx),
                )),
        )
}

/// `StructuredDialogModal` — the shell every settings dialog shares.
///
/// `max-w-md` default, `rounded-xl border shadow-2xl` on `--surface-0`, a
/// `px-5 py-4` header with an `IconChip` and a `p-5` body, and the action row
/// on the right.
fn dialog_shell(
    width: f32,
    icon_slug: &'static str,
    icon_tone: (u32, u32, u32),
    title: impl Into<SharedString>,
    subtitle: Option<SharedString>,
    body: Div,
    actions: Div,
) -> Div {
    let (icon_fg, icon_border, icon_bg) = icon_tone;
    div()
        .absolute()
        .top(px(0.))
        .left(px(0.))
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .px(px(12.))
        .bg(rgba(0x0000008c)) // bg-black/55
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(
            div()
                .w(px(width))
                .max_w(relative(1.0))
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(px(12.)) // rounded-xl
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .px(px(20.))
                        .py(px(16.))
                        .border_b_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .w(px(40.))
                                .h(px(40.))
                                .flex_shrink_0()
                                .rounded(px(10.))
                                .border_1()
                                .border_color(rgb(icon_border))
                                .bg(rgb(icon_bg))
                                .child(icon(icon_slug, 18., icon_fg)),
                        )
                        .child(
                            div()
                                .min_w(px(0.))
                                .flex_1()
                                .child(
                                    div()
                                        .text_size(px(16.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .line_height(relative(1.2))
                                        .text_color(rgb(TEXT_STRONG))
                                        .child(title.into()),
                                )
                                .when_some(subtitle, |this, subtitle| {
                                    this.child(
                                        div()
                                            .mt(px(2.))
                                            .text_size(px(12.))
                                            .line_height(relative(1.2))
                                            .text_color(rgb(TEXT_MUTED))
                                            .child(subtitle),
                                    )
                                }),
                        ),
                )
                .child(div().p(px(20.)).child(body))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(px(8.))
                        .px(px(20.))
                        .pb(px(20.))
                        .child(actions),
                ),
        )
}

fn restore_defaults_dialog(entity: WeakEntity<DragonFruit>) -> Div {
    let cancel_entity = entity.clone();
    let confirm_entity = entity;
    dialog_shell(
        448.0, // max-w-md
        "rotate-ccw",
        (0xf59e0b, mix(0xf59e0b, BORDER_SUBTLE, 0.45), mix(0xf59e0b, SURFACE_1, 0.9)),
        "Restore Defaults?",
        Some("This resets settings in this dialog to their default values.".into()),
        div().child(
            div()
                .flex()
                .flex_wrap()
                .text_size(px(12.))
                .line_height(relative(1.35))
                .text_color(rgb(TEXT_MUTED))
                .child("You can still review the changes before saving. Nothing is written until you click ")
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Apply"),
                )
                .child("."),
        ),
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(action_button(
                "restore-cancel",
                "Keep Current",
                None,
                16.0,
                SURFACE_1,
                BORDER_SUBTLE,
                TEXT_STRONG,
                SURFACE_2,
                move |cx| {
                    edit_and_notify(&cancel_entity, cx, |draft| {
                        draft.show_restore_confirm = false;
                    });
                },
            ))
            .child(action_button(
                "restore-confirm",
                "Restore Defaults",
                Some("rotate-ccw"),
                16.0,
                mix(ACCENT, SURFACE_1, 0.14),
                mix(ACCENT, BORDER_SUBTLE, 0.55),
                ACCENT,
                mix(ACCENT, SURFACE_1, 0.24),
                move |cx| {
                    edit_and_notify(&confirm_entity, cx, |draft| {
                        draft.restore_defaults();
                        draft.show_restore_confirm = false;
                    });
                },
            )),
    )
}

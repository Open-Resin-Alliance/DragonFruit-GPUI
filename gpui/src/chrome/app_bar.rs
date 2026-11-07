//! `src/components/layout/TopBar.tsx`, `src/components/controls/ViewTypeDropdown.tsx`,
//! `src/components/ui/SelectDropdown.tsx`, `src/features/shaders/mesh/types.ts`
//! (`MESH_SHADER_OPTIONS`), `src/components/ui/ContextMenu.tsx`, and the
//! `.ui-topbar*` / `.ui-button*` / `.ui-input` rules in `src/app/globals.css`.
//!
//! The bar is the window's title bar: it is frameless and the bar's own
//! background drags the window, while every control sits above that layer and
//! keeps its clicks.
//!
//! Integration: the parent renders this as an absolutely positioned strip at
//! the top of the window and owns the flags in [`AppBarState`].
//!
//! # PORT DEVIATIONS
//!
//! Diffed against the files above. Everything the platform cannot express, and
//! the native substitute:
//!
//! 1. `backdrop-filter: blur(24px)` on `.ui-topbar::before` and
//!    `.ui-topbar-blur` — GPUI has no backdrop filter. The bar paints the two
//!    translucent gradients only, so the scene behind it shows through
//!    unblurred.
//! 2. `mask-image` on the same pseudo-element (the bottom-edge fade to
//!    transparent) — not available; the vertical surface ramp's own alpha is
//!    the only fade.
//! 3. CSS transitions (`transition-colors`, `transition-opacity`,
//!    `transition-all duration-200`, `transition-transform`) — GPUI has no
//!    tweening, so every state change is instant.
//! 4. `rotate-180` on the printer badge chevron and on the `SelectDropdown`
//!    chevron — the icon swaps to `chevron-up` instead;
//!    `rotate-[-90deg]` on the "Show manager" chevron becomes `chevron-left`.
//! 5. `hover:-translate-y-[1px]` on an inactive workflow step — no transform in
//!    GPUI; the hover shadow is kept, the 1px lift is not.
//! 6. `tracking-[0.11em]` / `tracking-[0.01em]` / `tracking-wide` — GPUI has no
//!    letter-spacing, so all three render at the default tracking.
//! 7. `grayscale saturate-0` on the locked Printing step — no CSS filters; the
//!    step keeps its `opacity-45` plus the locked border/background mixes.
//! 8. `ui-topbar-monitor-active` / `ui-topbar-monitor-paused` keyframe
//!    animations — no CSS animation; the active/paused colours are the
//!    animations' 0% frames (`#22c55e` / `#facc15`).
//! 9. React portals — the menus are absolute children of the bar. The source's
//!    viewport clamp and upward flip need a measure pass; the bar sits at the
//!    top of the window, so every menu opens downwards.
//! 10. `title` / `aria-*` attributes — no native tooltip. `ui::tooltip` would
//!     need a wrapper element, which changes the bar's flex geometry.
//! 11. `useOutsideDismiss` / `useEscapeToClose` — the bar only toggles its own
//!     menus; global dismissal belongs to the parent.
//! 12. `hideWorkflowControls` (first-run onboarding) — the GPUI app has no
//!     onboarding, so the flag is not modelled.
//! 13. `isLightTheme` window-control tints — the GPUI app is dark-only, so the
//!     dark tints are the ones rendered.
//! 14. The `ResizeObserver` hysteresis for `showViewModeLabel` (appear at
//!     ≥155px per button, stay at ≥130px) — the bar derives `perButton` from
//!     the window width each frame and uses the 130px steady-state threshold.
//!     The right group is capped at `max-w-[430px]`, so the trigger's own width
//!     never feeds the measurement the asymmetry compensates for.
//! 15. `activePrinterProfile` / the profile store — [`PrinterBadge`] defaults to
//!     the source's empty-store rendering (`Printer` / `Select printer`) with no
//!     thumbnail and no fleet, so the printer button opens profile settings
//!     instead of the quick menu, exactly as `hasTopbarFleetUnits` does.
//! 16. `hasPrintingData` — passed in by the parent. The GPUI app has no slicing
//!     pipeline yet, so Printing stays locked as it does in a fresh source
//!     session.
//! 17. `ui::select_dropdown` does not expose the knobs this bar needs
//!     (`selectClassName`, `menuClassName`, `chevronClassName`, `hideChevron`,
//!     `selectedDisplay`, icon-only trigger). The bar keeps its own trigger for
//!     those; the app and fleet menus use `ui::context_menu_sized` /
//!     `ui::menu_surface`, whose row radius is the theme's `rounded-md` (10px)
//!     and whose hover is `color-mix(accent, surface-1 82%)`, exactly the
//!     source's `MenuRow`.
//!     `ui::select_dropdown` renders its trigger at 14px / 4px radius;
//!     `.ui-input` is unlayered CSS, so its 13px `--font-size-sm` and 6px
//!     `--radius-sm` beat the layered `text-sm` / `rounded-[4px]` utilities,
//!     and only the `!`-prefixed classes override it.
//! 18. The previous port's help button has no counterpart in `TopBar.tsx`; it
//!     is removed.
//! 19. Step icons carry `strokeWidth={2.5}` in the source; the generated SVGs
//!     bake the lucide default of 2, so the 12px glyphs are slightly lighter.
//! 20. `ViewTypeDropdown` passes no `leadingDisplay`, so `triggerLeading` falls
//!     back to the selected option's icon in *both* modes; in icon-only mode
//!     that icon therefore sits under the centred camera. Reproduced as-is.
//! 21. `custom-scrollbar` on the fleet list — GPUI draws the platform's own
//!     scrollbar; the class's thin accent styling has no counterpart.
//! 22. The re-slice warning (`warnBeforeProfileSettingsOpen` +
//!     `StructuredDialogModal`) has no counterpart: the port has no printer
//!     profiles to switch, so nothing can invalidate a slice.

use std::path::PathBuf;

use declarative_gpui::ui;
use gpui::{
    App, BoxShadow, Div, FontWeight, IntoElement, MouseButton, WeakEntity, WindowControlArea, div, img,
    linear_color_stop, linear_gradient, prelude::*, px, relative, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_core::theme::*;
use gpui_ui::ui::{ContextMenuEntry, context_menu_sized, menu_surface, menu_title};
use crate::{DragonFruit, Mode};
pub use gpui_core::state::MeshShaderType;

const WHITE: u32 = 0xffffff;
const BLACK: u32 = 0x000000;

// ── Mesh view types ─────────────────────────────────────────────────────────

/// `MESH_SHADER_OPTIONS` from `ViewTypeDropdown.tsx`: the dropdown's menu.
pub const MESH_SHADER_OPTIONS: [MeshShaderType; 6] = [
    MeshShaderType::SoftClay,
    MeshShaderType::OverhangHeatmap,
    MeshShaderType::OpaqueWireMesh,
    MeshShaderType::Wireframe,
    MeshShaderType::Xray,
    MeshShaderType::NormalDebug,
];

// ── Printer badge ───────────────────────────────────────────────────────────

/// One entry of `activePrinterProfile.networkFleet`.
#[derive(Clone, Debug)]
pub struct FleetUnit {
    pub id: String,
    /// `displayName || hostName || ipAddress || "Printer {id}"`.
    pub name: String,
    pub ip_address: String,
    /// `printerReachabilityByDeviceId[id] !== false`.
    pub online: bool,
}

/// The `activePrinterProfile` fields the bar reads.
#[derive(Clone, Debug, Default)]
pub struct PrinterBadge {
    /// `activePrinterProfile.name`; `None` renders the source's
    /// "Select printer" / "Select profile" fallback.
    pub profile_name: Option<String>,
    /// `activePrinterProfile.imageDataUrl`, already rasterised.
    pub thumbnail: Option<PathBuf>,
    /// `activePrinterProfile.networkFleet`, filtered to units with an IP.
    pub fleet: Vec<FleetUnit>,
    /// `activePrinterProfile.activeNetworkDeviceId`.
    pub active_device: Option<String>,
}

impl PrinterBadge {
    /// `topbarUsesFleetLabelOrder`: `networkFleet.length > 1`.
    fn uses_fleet_label_order(&self) -> bool {
        self.fleet.len() > 1
    }

    /// `topbarFleetPrinterName`.
    fn fleet_printer_name(&self) -> Option<&str> {
        if self.fleet.is_empty() {
            return None;
        }
        let preferred = self
            .active_device
            .as_deref()
            .and_then(|id| self.fleet.iter().find(|unit| unit.id == id))
            .or_else(|| self.fleet.iter().find(|unit| unit.online))
            .or_else(|| self.fleet.first())?;
        Some(preferred.name.as_str())
    }

    /// `topbarPrinterLabelTop`.
    fn label_top(&self) -> &str {
        if self.uses_fleet_label_order() {
            self.profile_name.as_deref().unwrap_or("Select profile")
        } else {
            "Printer"
        }
    }

    /// `topbarPrinterLabelBottom`.
    fn label_bottom(&self) -> &str {
        if self.uses_fleet_label_order() {
            self.fleet_printer_name().unwrap_or("No active printer")
        } else {
            self.profile_name.as_deref().unwrap_or("Select printer")
        }
    }

    /// `hasTopbarFleetUnits`: `topbarFleetUnits.length > 1`.
    fn has_fleet_units(&self) -> bool {
        self.fleet.len() > 1
    }
}

/// `showMonitorButton` plus its three state props.
#[derive(Clone, Copy, Debug, Default)]
pub struct MonitorButton {
    pub visible: bool,
    pub active: bool,
    pub paused: bool,
    pub offline: bool,
}

impl MonitorButton {
    /// `monitorButtonLabel`.
    fn label(&self) -> &'static str {
        if self.offline { "Offline" } else { "Monitor" }
    }

    /// `monitorButtonTone` overridden by the `ui-topbar-monitor-*` animation's
    /// first keyframe (see deviation 8).
    fn tone(&self) -> u32 {
        if self.offline {
            0xf87171
        } else if self.paused {
            0xfacc15
        } else if self.active {
            0x22c55e
        } else {
            TEXT_STRONG
        }
    }
}

// ── State ───────────────────────────────────────────────────────────────────

/// A per-frame snapshot of everything the bar draws from.
///
/// The parent owns the real state on `DragonFruit`; it builds this each frame
/// and writes any flag the bar toggles back on the entity.
pub struct AppBarState {
    /// Highlighted workflow step.
    pub mode: Mode,
    /// Drives `hasModels`, and with it the locked workflow steps.
    pub triangles: usize,
    /// `isSlicingBusy`: dims and disables the bar's actions.
    pub loading: bool,
    /// `hasPrintingData`: unlocks the Printing step.
    pub has_printing_data: bool,
    /// `viewType` — the persisted mesh shader the dropdown shows.
    pub view_type: MeshShaderType,
    /// `overhangViewActive`.
    pub overhang: bool,
    pub show_settings: bool,
    pub show_printer_menu: bool,
    pub show_app_menu: bool,
    /// The `SelectDropdown` menu under the view-mode trigger.
    pub show_view_menu: bool,
    /// Rasterised `simple_icon.svg`; `None` falls back to the pink "D" tile.
    pub logo: Option<PathBuf>,
    /// `window.innerWidth`, for the source's `ResizeObserver` thresholds.
    pub window_width: f32,
    pub maximized: bool,
    pub printer: PrinterBadge,
    pub monitor: MonitorButton,
}

impl Default for AppBarState {
    fn default() -> Self {
        Self {
            mode: Mode::Prepare,
            triangles: 0,
            loading: false,
            has_printing_data: false,
            view_type: MeshShaderType::SoftClay,
            overhang: false,
            show_settings: false,
            show_printer_menu: false,
            show_app_menu: false,
            show_view_menu: false,
            logo: None,
            window_width: 0.0,
            maximized: false,
            printer: PrinterBadge::default(),
            monitor: MonitorButton::default(),
        }
    }
}

// ── Shadows ─────────────────────────────────────────────────────────────────

/// A CSS `box-shadow` layer: `offset-x offset-y blur spread rgba(...)`.
fn shadow(x: f32, y: f32, blur: f32, spread: f32, color: u32) -> BoxShadow {
    BoxShadow::new(px(x), px(y), rgba(color).into())
        .blur_radius(px(blur))
        .spread_radius(px(spread))
}

/// A CSS `box-shadow` layer with `inset`.
fn inset_shadow(x: f32, y: f32, blur: f32, spread: f32, color: u32) -> BoxShadow {
    shadow(x, y, blur, spread, color).inset()
}

// ── The bar ─────────────────────────────────────────────────────────────────

/// `TopBar.tsx` + `.ui-topbar`.
///
/// `entity` receives every state change the bar initiates (mode, menus, view
/// type, overhang, settings, scene actions).
pub fn app_bar(state: AppBarState, entity: WeakEntity<DragonFruit>) -> impl IntoElement {
    let AppBarState {
        mode,
        triangles,
        loading,
        has_printing_data,
        view_type,
        overhang,
        // Write-only: the bar opens the modal, the parent renders it, and the
        // source's `isSettingsOpen` never styles the trigger.
        show_settings: _,
        show_printer_menu,
        show_app_menu,
        show_view_menu,
        logo,
        window_width,
        maximized,
        printer,
        monitor,
    } = state;

    let actions_disabled = loading;
    let has_models = triangles > 0;

    // The source measures the steps container with a `ResizeObserver` and
    // derives both responsive flags from it. The container's width follows from
    // the bar's flex geometry: `px-4` on the bar, `gap-3` between the three
    // groups, `flex-1 max-w-[430px]` on the two side groups, `px-2` on the
    // centre, and the source's own `gapPx = 4` estimate for the four steps.
    let content = (window_width - 8.0).max(0.0);
    let share = ((content - 24.0) / 3.0).max(0.0);
    let side = share.min(430.0);
    let centre = (content - 24.0 - side * 2.0).max(0.0);
    let steps_width = (centre - 16.0).max(0.0);
    let per_button = (steps_width - 12.0) / 4.0;
    let hide_badges = per_button < 120.0;
    // See deviation 14: the 130px steady-state threshold.
    let show_view_mode_label = per_button >= 130.0;

    // ── Left group: menu button, divider, printer badge, monitor ────────────
    let logo_entity = entity.clone();
    let logo_child = match logo {
        Some(path) => img(path).w(px(28.)).h(px(28.)).into_any_element(),
        None => div()
            .w(px(28.))
            .h(px(28.))
            .rounded(px(7.))
            .bg(rgb(ACCENT))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(14.))
            .font_weight(FontWeight::BOLD)
            .text_color(rgb(ACCENT_CONTRAST))
            .child("D")
            .into_any_element(),
    };

    // `inline-flex h-10 w-10 items-center justify-center rounded-md
    // transition-colors`, `color-mix(accent, transparent 80%)` when open.
    let app_menu_button = ui! {
        div(
            id = "app-menu-button",
            w_40, h_40, flex, items_center, justify_center,
            rounded_6,
            when(show_app_menu, bg = rgba(0xec2a7733)),
            when(!actions_disabled, cursor_pointer),
            on_click = move |_, _, cx| {
                if actions_disabled {
                    return;
                }
                logo_entity
                    .update(cx, |this, cx| {
                        this.show_app_menu = !this.show_app_menu;
                        this.show_printer_menu = false;
                        this.show_view_menu = false;
                        cx.notify();
                    })
                    .ok();
            },
        ) {
            { logo_child }
        }
    };

    let printer_entity = entity.clone();
    let has_fleet_units = printer.has_fleet_units();
    let printer_chevron = if show_printer_menu {
        "chevron-up"
    } else {
        "chevron-down"
    };
    let printer_thumbnail = match &printer.thumbnail {
        Some(path) => img(path.clone())
            .w(px(32.))
            .h(px(32.))
            .into_any_element(),
        None => icon("printer", 16., TEXT_MUTED).into_any_element(),
    };
    let printer_label_top = printer.label_top().to_owned();
    let printer_label_bottom = printer.label_bottom().to_owned();
    let fleet_label_order = printer.uses_fleet_label_order();

    // `group inline-flex h-10 max-w-[300px] items-center gap-2 rounded-md px-2`.
    let printer_button = ui! {
        div(
            id = "printer-menu-button",
            h_40, max_w_300, flex, items_center, gap_8, px_8,
            rounded_6,
            when(!actions_disabled, cursor_pointer),
            on_click = move |_, _, cx| {
                if actions_disabled {
                    return;
                }
                // `hasTopbarFleetUnits` decides between the quick menu and the
                // printer profile settings.
                printer_entity
                    .update(cx, |this, cx| {
                        if has_fleet_units {
                            this.show_printer_menu = !this.show_printer_menu;
                            this.show_app_menu = false;
                            this.show_view_menu = false;
                        } else {
                            this.show_printer_menu = false;
                            this.show_app_menu = false;
                            this.show_view_menu = false;
                            this.show_settings = true;
                        }
                        cx.notify();
                    })
                    .ok();
            },
        ) {
            // `h-8 w-8 rounded-sm`, `color-mix(surface-1, transparent 6%)`.
            div(
                w_32, h_32, flex, items_center, justify_center,
                overflow_hidden, flex_shrink_0, rounded_4, bg = rgba(0x1a1b21f0),
            ) {
                { printer_thumbnail }
            }
            div(flex, flex_col, items_start, min_w_0, gap_2, line_height = relative(1.0)) {
                div(
                    text_11, text_color = rgb(TEXT_MUTED),
                    when(fleet_label_order, truncate),
                ) {
                    text(printer_label_top)
                }
                div(text_12, font_semibold, truncate, text_color = rgb(TEXT_STRONG)) {
                    text(printer_label_bottom)
                }
            }
            { icon(printer_chevron, 14., mix(TEXT_MUTED, WHITE, 0.92)) }
        }
    };

    // `showMonitorButton && <button class="group inline-flex h-10 items-center
    // gap-1.5 rounded-md px-2">`.
    let monitor_button = if monitor.visible {
        let tone = monitor.tone();
        let monitor_label = monitor.label();
        let monitor_entity = entity.clone();
        ui! {
            div(
                id = "monitor-button",
                h_40, flex, items_center, gap_6, px_8, rounded_6,
                when(!actions_disabled, cursor_pointer),
                on_click = move |_, _, cx| {
                    if actions_disabled {
                        return;
                    }
                    monitor_entity
                        .update(cx, |this, cx| {
                            this.show_monitor = true;
                            cx.notify();
                        })
                        .ok();
                },
            ) {
                { icon("activity", 14., tone) }
                div(text_11, font_semibold, text_color = rgb(tone)) {
                    text(monitor_label)
                }
            }
        }
        .into_any_element()
    } else {
        div().into_any_element()
    };

    let left = ui! {
        div(
            relative, flex, flex_1, max_w_430, items_center, gap_10, pr_16, py_6,
            when(actions_disabled, opacity_45),
        ) {
            { app_menu_button }
            // `h-6 w-px mx-0.5 shrink-0`, `color-mix(border-subtle,
            // transparent 24%)`.
            div(w_px, h_24, mx_2, flex_shrink_0, bg = rgba(0x272a33c2)) {}
            { printer_button }
            { monitor_button }
        }
    };

    // ── Centre: the four workflow steps ─────────────────────────────────────
    let steps = ui! {
        div(relative, flex, min_w_0, flex_1, justify_center, px_8) {
            div(relative, w_full, when(actions_disabled, opacity_45)) {
                div(flex, items_center, justify_center, gap_8) {
                    { step_button(Mode::Prepare, mode, false, actions_disabled, hide_badges, entity.clone()) }
                    { step_button(Mode::Support, mode, !has_models, actions_disabled, hide_badges, entity.clone()) }
                    { step_button(Mode::Export, mode, !has_models, actions_disabled, hide_badges, entity.clone()) }
                    { step_button(Mode::Printing, mode, !has_models || !has_printing_data, actions_disabled, hide_badges, entity.clone()) }
                }
            }
        }
    };

    // ── Right group: view mode, overhang, settings, window controls ─────────
    let view_mode = view_type_dropdown(
        view_type,
        show_view_menu,
        actions_disabled,
        show_view_mode_label,
        entity.clone(),
    );

    let overhang_entity = entity.clone();
    let overhang_button = toolbar_button(
        "overhang-toggle",
        "alert-diamond-custom",
        if overhang {
            ToolbarTone::Primary
        } else {
            ToolbarTone::Secondary
        },
        actions_disabled,
        move |cx| {
            overhang_entity
                .update(cx, |this, cx| {
                    this.overhang_view = !this.overhang_view;
                    cx.notify();
                })
                .ok();
        },
    );

    let settings_entity = entity.clone();
    let settings_button = toolbar_button(
        "settings-button",
        "settings",
        ToolbarTone::Secondary,
        actions_disabled,
        move |cx| {
            gpui_core::log::log("appbar: settings click");
            settings_entity
                .update(cx, |this, cx| {
                    this.show_settings = true;
                    cx.notify();
                })
                .ok();
        },
    );

    let right = ui! {
        div(relative, flex, flex_1, max_w_430, items_center, justify_end, gap_8, pr_3) {
            div(flex, items_center, gap_8, when(actions_disabled, opacity_45)) {
                { view_mode }
                { overhang_button }
                { settings_button }
            }
            // `ml-1 flex items-center gap-1`.
            div(ml_4, flex, items_center, gap_4) {
                { window_button(
                    "caption-min",
                    CaptionAction::Min,
                    0xf4bf4f,
                    "minimize-2",
                    TintColor::Mix16,
                    0.86,
                ) }
                { window_button(
                    "caption-max",
                    CaptionAction::Max,
                    0x40c463,
                    if maximized { "square" } else { "maximize-2" },
                    TintColor::Mix16,
                    0.86,
                ) }
                { window_button("caption-close", CaptionAction::Close, 0xff6b6b, "x", TintColor::Mix18, 0.88) }
            }
        }
    };

    // ── Popups ──────────────────────────────────────────────────────────────
    // `openAppMenu`: `{ x: rect.left, y: rect.bottom + 6 }` under the 40px
    // button that starts at the bar's 4px left padding and 3px top padding.
    let app_menu = if show_app_menu {
        app_menu(has_models, actions_disabled, entity.clone()).into_any_element()
    } else {
        div().into_any_element()
    };

    // `openPrinterQuickMenu`: the printer badge starts after the logo (40px),
    // `gap-2.5` (10), the divider (`w-px` + `mx-0.5`, 5) and `gap-2.5` (10).
    let printer_menu = if show_printer_menu && has_fleet_units {
        printer_menu(&printer, actions_disabled, entity.clone()).into_any_element()
    } else {
        div().into_any_element()
    };

    // ── Shell ───────────────────────────────────────────────────────────────
    ui! {
        div(
            id = "app-bar",
            absolute, top_0, left_0, right_0,
            h = px(TOPBAR_HEIGHT),
            flex, items_center, gap_12,
            // 36px controls + 6px top and bottom.
            h = px(48.),
            px_4, pt_1p5, pb_1p5,
            on_mouse_down = move |_, window, _| {
                gpui_core::log::log("appbar: drag root");
                start_window_drag(window);
            },
        ) {
            { bar_surface() }
            { left }
            { steps }
            { right }
        }
    }
    .when(show_app_menu, |this| this.child(app_menu))
    .when(show_printer_menu, |this| this.child(printer_menu))
}

/// `.ui-topbar::before`: the two background gradients.
///
/// GPUI's `linear_gradient` takes exactly two stops, so each three-stop CSS ramp
/// is split at its middle stop into two adjacent halves — the same
/// piecewise-linear result CSS interpolates between stops.
fn bar_surface() -> Div {
    // `linear-gradient(to bottom, color-mix(surface-0, transparent 6%) 0%,
    // color-mix(surface-0, transparent 10%) 50%, color-mix(surface-0,
    // transparent 24%) 100%)`.
    // `linear-gradient(90deg, color-mix(topbar-accent, transparent 93%) 0%,
    // color-mix(topbar-accent, transparent 97%) 45%, color-mix(topbar-accent,
    // transparent 94%) 100%)`.
    let accent_left = 0xec2a7712; // 93% transparent
    let accent_mid = 0xec2a7708; // 97% transparent
    let accent_right = 0xec2a770f; // 94% transparent

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        // The surface gradient without the source's bottom fade: the bar ends
        // at the content, so the scene shows through below it.
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .h(relative(0.5))
                .bg(linear_gradient(
                    180.0,
                    linear_color_stop(rgba(0x111216f0), 0.0),
                    linear_color_stop(rgba(0x111216e6), 1.0),
                )),
        )
        .child(
            div()
                .absolute()
                .top(relative(0.5))
                .left_0()
                .right_0()
                .bottom_0()
                .bg(linear_gradient(
                    180.0,
                    linear_color_stop(rgba(0x111216e6), 0.0),
                    linear_color_stop(rgba(0x111216c2), 1.0),
                )),
        )
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left_0()
                .w(relative(0.45))
                .bg(linear_gradient(
                    90.0,
                    linear_color_stop(rgba(accent_left), 0.0),
                    linear_color_stop(rgba(accent_mid), 1.0),
                )),
        )
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(relative(0.45))
                .right_0()
                .bg(linear_gradient(
                    90.0,
                    linear_color_stop(rgba(accent_mid), 0.0),
                    linear_color_stop(rgba(accent_right), 1.0),
                )),
        )
}

/// One workflow step: `h-[32px] flex-1 min-w-[90px] max-w-[190px]
/// rounded-[5.5px] border px-1.5 py-1.5`.
fn step_button(
    mode: Mode,
    active: Mode,
    locked: bool,
    disabled: bool,
    hide_badges: bool,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let is_active = mode == active;
    let dimmed = disabled || locked;
    let printing_locked = mode == Mode::Printing && locked;
    let icon_name = match mode {
        Mode::Prepare => "folder-open",
        Mode::Support => "anchor",
        Mode::Export => "upload",
        Mode::Printing => "printer",
    };

    let (background, border) = if is_active {
        // `color-mix(accent, white 8%)` border, `color-mix(accent,
        // surface-0 84%)` fill.
        (
            gpui::rgb(mix(ACCENT, SURFACE_0, 0.16)),
            mix(ACCENT, WHITE, 0.92),
        )
    } else if printing_locked {
        // `color-mix(border-subtle, black 30%)` border, `color-mix(surface-2,
        // black 12%)` fill.
        (
            gpui::rgb(mix(SURFACE_2, BLACK, 0.88)),
            mix(BORDER_SUBTLE, BLACK, 0.70),
        )
    } else {
        // `border-subtle` border, `color-mix(surface-1, transparent 4%)`.
        (gpui::rgba(0x1a1b21f5), BORDER_SUBTLE)
    };

    let badge_icon = if is_active { ACCENT } else { TEXT_MUTED };
    // `shadow-[0_6px_16px_rgba(0,0,0,0.25)]` on the active step;
    // `hover:shadow-[0_6px_14px_rgba(0,0,0,0.18)]` on an idle one.
    let active_shadow = || shadow(0.0, 6.0, 16.0, 0.0, 0x00000040);
    let hover_shadow = || shadow(0.0, 6.0, 14.0, 0.0, 0x0000002e);

    let label = mode.label();
    let step_id = gpui::ElementId::named_usize("step", mode.step() as usize);

    let mut step = div()
        .id(step_id)
        .relative()
        .h(px(32.0))
        .flex_1()
        .min_w(px(90.0))
        .max_w(px(190.0))
        .flex()
        .items_center()
        .gap(px(6.0))
        .px(px(6.0))
        .py(px(6.0))
        .rounded(px(5.5))
        .border_1()
        .border_color(rgb(border))
        .bg(background);

    if is_active {
        step = step.shadow(vec![active_shadow()]);
    } else if !dimmed {
        step = step.hover(move |style| style.shadow(vec![hover_shadow()]));
    }

    if dimmed {
        step = step.opacity(0.45);
    }
    if disabled {
        step = step.cursor_not_allowed();
    } else {
        step = step
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(move |_, _, cx| {
            entity
                .update(cx, |this, cx| {
                    this.mode = mode;
                    cx.notify();
                })
                .ok();
        });
    }

    if !hide_badges {
        step = step.child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .w(px(20.0))
                .h(px(20.0))
                .rounded_full()
                .when(!is_active, |el| el.bg(rgb(SURFACE_2)))
                .child(icon(icon_name, 12.0, badge_icon)),
        );
    }

    // `absolute left-1/2 -translate-x-1/2 whitespace-nowrap text-center
    // text-xs font-bold leading-none tracking-[0.01em]`. Centring the text in
    // the full button width is the same box as `left: 50%` plus a half-width
    // pull-back for a single line.
    step = step.child(
        div()
            .absolute()
            .left(px(0.0))
            .right(px(0.0))
            .text_center()
            .whitespace_nowrap()
            .text_size(px(12.0))
            .font_weight(FontWeight::BOLD)
            .line_height(px(12.0))
            .text_color(rgb(TEXT_STRONG))
            .child(label),
    );

    if printing_locked && !hide_badges {
        step = step.child(
            div()
                .ml_auto()
                .flex_shrink_0()
                .child(icon("lock", 12.0, TEXT_MUTED)),
        );
    }

    step
}

/// The `Button` variants the bar's square icon buttons use.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ToolbarTone {
    /// `variant="secondary"`.
    Secondary,
    /// `variant="primary"` — the active overhangs toggle.
    Primary,
}

/// `Button` with `size="md"` (`!h-9 px-3 text-xs`) and `className="!p-2"`.
///
/// `.ui-button` is unlayered CSS, so its `font-size: var(--font-size-md)` (14px)
/// and `line-height: 1` beat the layered `text-xs`; only the `!`-prefixed
/// `!h-9` and `!p-2` override it. The box is therefore 36px tall and
/// 8 + 16 + 8 + 2 borders = 34px wide.
fn toolbar_button(
    id: &'static str,
    icon_name: &'static str,
    tone: ToolbarTone,
    disabled: bool,
    on_click: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    let (background, border, foreground, hover) = match tone {
        ToolbarTone::Primary => (
            PRIMARY_BUTTON_SURFACE,
            mix(PRIMARY_BUTTON_SURFACE, WHITE, 0.86),
            ACCENT_CONTRAST,
            mix(PRIMARY_BUTTON_SURFACE, BLACK, 0.88),
        ),
        ToolbarTone::Secondary => (SURFACE_1, BORDER_SUBTLE, TEXT_STRONG, SURFACE_2),
    };

    let mut button = div()
        .id(id)
        .h(px(36.0))
        .flex()
        .items_center()
        .justify_center()
        .p(px(8.0))
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .text_size(px(FONT_MD))
        .font_weight(FontWeight::SEMIBOLD)
        .line_height(px(FONT_MD))
        .child(icon(icon_name, 16.0, foreground));

    if disabled {
        button = button.opacity(0.55).cursor_not_allowed();
    } else {
        button = button
            .cursor_pointer()
            .hover(move |style| style.bg(rgb(hover)))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(move |_, _, cx| on_click(cx));
    }

    button
}

/// How the window-control glyph is tinted (`color-mix(tint, text-strong N%)`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum TintColor {
    /// `color: var(--text-strong)`.
    Plain,
    /// `color-mix(tint, var(--text-strong) 16%)`.
    Mix16,
    /// `color-mix(tint, var(--text-strong) 18%)`.
    Mix18,
}

/// The frameless window's caption buttons: `inline-flex h-8 w-8 items-center
/// justify-center rounded-[var(--radius-sm)] border transition-colors`.
///
/// `background_weight` is the `surface-1` share of `color-mix(tint,
/// surface-1 N%)` — 86% for minimize/maximize, 88% for close.
fn window_button(
    id: &'static str,
    action: CaptionAction,
    tint: u32,
    icon_name: &'static str,
    color: TintColor,
    background_weight: f32,
) -> impl IntoElement {
    // `color-mix(in srgb, tint, var(--text-strong) 16%)` keeps 84% of the
    // tint; the percentage is the second colour's share.
    let foreground = match color {
        TintColor::Plain => TEXT_STRONG,
        TintColor::Mix16 => mix(tint, TEXT_STRONG, 0.84),
        TintColor::Mix18 => mix(tint, TEXT_STRONG, 0.82),
    };

    ui! {
        div(
            id = id,
            w_32, h_32, flex, items_center, justify_center,
            rounded_6, border, cursor_pointer,
            border_color = rgb(mix(tint, BORDER_SUBTLE, 0.45)),
            bg = rgb(mix(tint, SURFACE_1, 1.0 - background_weight)),
            on_mouse_down = move |_, _, cx| cx.stop_propagation(),
            on_click = move |_, window, _| match action {
                CaptionAction::Min => window.minimize_window(),
                CaptionAction::Max => toggle_maximize(window),
                CaptionAction::Close => window.remove_window(),
            },
        ) {
            { icon(icon_name, 16., foreground) }
        }
    }
}

// ── View type dropdown ──────────────────────────────────────────────────────

/// `ViewTypeDropdown.tsx` on top of `SelectDropdown.tsx`.
///
/// Labelled trigger: `.ui-input rounded-[4px] !h-9 !py-1.5 !pl-2 !pr-7 text-xs`
/// with the selected option's icon at the left and the chevron at
/// `right-1.5`. Icon-only trigger: `!h-9 !w-9 !p-0`, no chevron, the label
/// transparent and a centred camera. The menu is `!w-64` (256px), pinned to the
/// trigger's right edge, 6px below it.
fn view_type_dropdown(
    value: MeshShaderType,
    open: bool,
    disabled: bool,
    show_label: bool,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let icon_only = !show_label;
    let toggle_entity = entity.clone();

    let (border, background) = if disabled {
        (BORDER_SUBTLE, SURFACE_0)
    } else if open {
        // `border-color: color-mix(accent, border-subtle 18%)`.
        (mix(ACCENT, BORDER_SUBTLE, 0.82), SURFACE_0)
    } else {
        (BORDER_SUBTLE, SURFACE_0)
    };

    // `.ui-input` is unlayered CSS, so it wins over the layered utilities the
    // trigger also carries: `rounded-[4px]`, `text-sm` and `leading-none` all
    // lose to its `border-radius: var(--radius-sm)` (6px), `font-size:
    // var(--font-size-sm)` (13px) and `line-height: 1.3`. Only the `!`-prefixed
    // classes (`!h-9`, `!py-1.5`, `!pl-2`, `!pr-7`, `!p-0`, `!w-9`) override it.
    let mut trigger = div()
        .id("view-type-trigger")
        .h(px(36.0))
        .flex()
        .items_center()
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .text_size(px(FONT_SM))
        .line_height(px(FONT_SM * 1.3))
        .text_color(rgb(TEXT_STRONG));

    if icon_only {
        trigger = trigger.w(px(36.0)).p(px(0.0));
    } else {
        trigger = trigger.pl(px(8.0)).pr(px(28.0)).py(px(6.0));
    }

    if open && !disabled {
        // `box-shadow: 0 0 0 1px color-mix(accent, transparent 72%) inset`.
        trigger = trigger.shadow(vec![inset_shadow(0.0, 0.0, 0.0, 1.0, 0xec2a7747)]);
    }

    // `triggerLeading = leadingDisplay ?? selectedOption.icon` — present in
    // both modes (deviation 20).
    trigger = trigger.child(
        div()
            .mr(px(6.0))
            .w(px(18.0))
            .h(px(18.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .child(icon(value.icon(), 14.0, TEXT_STRONG)),
    );

    trigger = trigger.child(
        div()
            .flex_1()
            .min_w(px(0.0))
            .truncate()
            .when(icon_only, |el| el.text_color(rgba(0x00000000)))
            .child(value.label()),
    );

    if !icon_only {
        trigger = trigger.child(
            div()
                .absolute()
                .right(px(6.0))
                .top(px(0.0))
                .bottom(px(0.0))
                .flex()
                .items_center()
                .child(icon(
                    if open { "chevron-up" } else { "chevron-down" },
                    14.0,
                    TEXT_MUTED,
                )),
        );
    } else {
        // `selectedDisplay` centred over the transparent label.
        trigger = trigger.child(
            div()
                .absolute()
                .left(px(0.0))
                .right(px(0.0))
                .top(px(0.0))
                .bottom(px(0.0))
                .flex()
                .items_center()
                .justify_center()
                .child(icon("camera", 16.0, TEXT_STRONG)),
        );
    }

    if disabled {
        trigger = trigger.opacity(0.55).cursor_not_allowed();
    } else {
        trigger = trigger.cursor_pointer().on_click(move |_, _, cx| {
            toggle_entity
                .update(cx, |this, cx| {
                    this.show_view_menu = !this.show_view_menu;
                    this.show_app_menu = false;
                    this.show_printer_menu = false;
                    cx.notify();
                })
                .ok();
        });
    }

    let mut container = div().relative().flex().flex_col().child(trigger);

    if open && !disabled {
        container = container.child(view_type_menu(value, entity));
    }

    container
}

/// An icon that takes its colour from the surrounding text style instead of
/// carrying one of its own, so a parent's `group_hover` can re-tint it
/// (`SelectDropdown`'s option icons brighten with the row).
fn tinted_icon(name: &str, size: f32) -> impl IntoElement {
    let path = format!("{}/assets/icons/{name}.svg", env!("CARGO_MANIFEST_DIR"));
    gpui::svg().external_path(path).w(px(size)).h(px(size))
}

/// The `SelectDropdown` menu for the view types: `fixed z-[9999] rounded-[4px]
/// border shadow-xl !w-64`, `color-mix(accent, border-subtle 82%)` border and
/// `color-mix(surface-0, black 6%)` fill.
fn view_type_menu(value: MeshShaderType, entity: WeakEntity<DragonFruit>) -> impl IntoElement {
    let mut list = div().flex().flex_col();

    for (index, option) in MESH_SHADER_OPTIONS.iter().enumerate() {
        let option = *option;
        let is_selected = option == value;
        let last = index == MESH_SHADER_OPTIONS.len() - 1;
        // `border-bottom-color` is transparent on the last row of a group,
        // otherwise `color-mix(border-subtle, transparent 16%)`.
        let rule = if last {
            0x00000000
        } else {
            0x272a33d6
        };

        let (background, foreground, icon_color) = if is_selected {
            // `color-mix(accent, surface-0 86%)`.
            (mix(ACCENT, SURFACE_0, 0.14), TEXT_STRONG, ACCENT)
        } else {
            (0x00000000, TEXT_MUTED, TEXT_MUTED)
        };

        let select_entity = entity.clone();
        let mut icon_well = div()
            .w(px(14.0))
            .h(px(14.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .text_color(rgb(icon_color))
            .child(tinted_icon(option.icon(), 14.0));
        if !is_selected {
            // `isSelected ? accent : (isHovered ? text-strong : text-muted)`.
            icon_well = icon_well.group_hover("view-type-option", |style| {
                style.text_color(rgb(TEXT_STRONG))
            });
        }

        let mut row = div()
            .id(gpui::ElementId::named_usize("view-type-option", index))
            .group("view-type-option")
            .w_full()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(12.0))
            .py(px(8.0))
            .text_size(px(14.0))
            .text_left()
            .border_b(px(1.0))
            .border_color(rgba(rule))
            .bg(rgba(background))
            .text_color(rgb(foreground))
            .cursor_pointer()
            .child(icon_well)
            .child(
                div()
                    .min_w(px(0.0))
                    .truncate()
                    // `text-[0.95em]` of the row's 14px.
                    .text_size(px(13.3))
                    .child(option.label()),
            )
            .on_click(move |_, _, cx| {
                select_entity
                    .update(cx, |this, cx| {
                        this.view_type = option;
                        this.show_view_menu = false;
                        cx.notify();
                    })
                    .ok();
            });

        if !is_selected {
            // `color-mix(surface-2, transparent 18%)` fill, `--text-strong`
            // text and icon once hovered.
            row = row.hover(|style| {
                style
                    .bg(rgba(0x23252ed1))
                    .text_color(rgb(TEXT_STRONG))
            });
        }

        list = list.child(row);
    }

    div()
        .absolute()
        .top(relative(1.0))
        .mt(px(6.0))
        .right(px(0.0))
        .w(px(256.0))
        .rounded(px(4.0))
        .border_1()
        .border_color(rgb(mix(ACCENT, BORDER_SUBTLE, 0.18)))
        .bg(rgb(mix(SURFACE_0, BLACK, 0.94)))
        .flex()
        .flex_col()
        .shadow(vec![
            shadow(0.0, 14.0, 34.0, 0.0, 0x00000075),
            shadow(0.0, 4.0, 12.0, 0.0, 0x00000052),
        ])
        .child(list)
}

// ── Menus ───────────────────────────────────────────────────────────────────

/// The `DragonFruit` app menu under the logo button.
///
/// Entries and their disabled rules come from `appMenuEntries`; a handler the
/// parent does not provide leaves the row disabled, exactly as the source's
/// `!onSaveScene` checks do. The surface, heading, row geometry and hover fill
/// are `ui::context_menu`'s.
fn app_menu(
    has_models: bool,
    actions_disabled: bool,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let mut entries = Vec::with_capacity(7);

    let mut new_scene = ContextMenuEntry::new("menu-new-scene", "New scene").with_icon("file-plus2");
    if actions_disabled || !has_models {
        new_scene = new_scene.disabled();
    }
    entries.push(new_scene);

    // `Save scene` / `Save scene as…` / `Open scene…` / `Import scene…` have no
    // handler in this build, so they render disabled.
    entries.push(
        ContextMenuEntry::new("menu-save-scene", "Save scene")
            .with_icon("save")
            .starts_group()
            .disabled(),
    );
    entries.push(
        ContextMenuEntry::new("menu-save-scene-as", "Save scene as…")
            .with_icon("save-all")
            .disabled(),
    );
    entries.push(
        ContextMenuEntry::new("menu-open-scene", "Open scene…")
            .with_icon("folder-open")
            .disabled(),
    );

    let mut import_mesh =
        ContextMenuEntry::new("menu-import-mesh", "Import mesh…")
            .with_icon("upload")
            .starts_group();
    if actions_disabled {
        import_mesh = import_mesh.disabled();
    }
    entries.push(import_mesh);
    entries.push(
        ContextMenuEntry::new("menu-import-scene", "Import scene…")
            .with_icon("folder-input")
            .disabled(),
    );
    entries.push(
        ContextMenuEntry::new("menu-close-program", "Close program")
            .with_icon("power")
            .starts_group(),
    );

    context_menu_sized(
        &entries,
        Some("DRAGONFRUIT".into()),
        192.0,
        move |id, _, cx| match id {
            "menu-new-scene" => {
                entity
                    .update(cx, |this, cx| {
                        this.mesh = None;
                        this.supports.borrow_mut().clear();
                        this.status = "No model loaded".to_owned();
                        this.mode = Mode::Prepare;
                        this.show_app_menu = false;
                        cx.notify();
                    })
                    .ok();
            }
            "menu-import-mesh" => {
                entity
                    .update(cx, |this, cx| {
                        this.show_app_menu = false;
                        this.open_stl(cx);
                    })
                    .ok();
            }
            "menu-close-program" => cx.quit(),
            _ => {}
        },
    )
    .absolute()
    .top(px(49.0))
    .left(px(4.0))
    // The bar root starts a window drag on mouse-down; a menu press is not one.
    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
}

/// The printer quick menu: `fixed z-[121] w-[280px] rounded-lg border p-1.5
/// shadow-xl backdrop-blur-sm`.
///
/// Only rendered when `hasTopbarFleetUnits`; each row is the fleet unit's
/// thumbnail, name, `ip • Online/Offline` line and the "Active" badge.
fn printer_menu(
    printer: &PrinterBadge,
    actions_disabled: bool,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let mut list = div()
        .id("fleet-unit-list")
        .max_h(px(260.0))
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .gap(px(2.0));

    for (index, unit) in printer.fleet.iter().enumerate() {
        let active = printer.active_device.as_deref() == Some(unit.id.as_str());
        let offline = !unit.online;

        let (border, background, foreground) = if offline {
            // `color-mix(border-subtle, black 18%)`, `color-mix(surface-1,
            // black 8%)`, `opacity: 0.55`.
            (
                mix(BORDER_SUBTLE, BLACK, 0.82),
                mix(SURFACE_1, BLACK, 0.92),
                TEXT_MUTED,
            )
        } else if active {
            // `color-mix(accent, border-subtle 30%)`, `color-mix(accent,
            // surface-1 89%)`, `--text-strong`.
            (
                mix(ACCENT, BORDER_SUBTLE, 0.70),
                mix(ACCENT, SURFACE_1, 0.11),
                TEXT_STRONG,
            )
        } else {
            (BORDER_SUBTLE, SURFACE_1, TEXT_MUTED)
        };

        let thumbnail = match &printer.thumbnail {
            Some(path) => img(path.clone())
                .w(px(24.))
                .h(px(24.))
                .into_any_element(),
            None => icon("printer", 14., TEXT_MUTED).into_any_element(),
        };

        let mut row = div()
            .id(gpui::ElementId::named_usize("fleet-unit", index))
            .w_full()
            .flex()
            .items_center()
            .gap(px(8.0))
            .rounded(px(ROUNDED_MD))
            .border_1()
            .border_color(rgb(border))
            .bg(rgb(background))
            .px(px(8.0))
            .py(px(6.0))
            .text_left()
            .text_size(px(12.0))
            .text_color(rgb(foreground))
            .child(
                div()
                    .w(px(24.0))
                    .h(px(24.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .rounded(px(ROUNDED_SM))
                    .bg(rgba(0x1a1b21f0))
                    .child(thumbnail),
            )
            .child(
                div()
                    .min_w(px(0.0))
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .truncate()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(unit.name.clone()),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(px(10.0))
                            .text_color(rgb(TEXT_MUTED))
                            .child(format!(
                                "{} • {}",
                                if unit.ip_address.is_empty() {
                                    "No IP"
                                } else {
                                    unit.ip_address.as_str()
                                },
                                if offline { "Offline" } else { "Online" }
                            )),
                    ),
            );

        if active {
            // `text-[10px] rounded-full border px-1.5 py-0.5`,
            // `color-mix(accent-secondary, border-subtle 45%)` border,
            // `color-mix(accent-secondary, surface-1 92%)` fill.
            row = row.child(
                div()
                    .text_size(px(10.0))
                    .rounded_full()
                    .border_1()
                    .border_color(rgb(mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.55)))
                    .bg(rgb(mix(ACCENT_SECONDARY, SURFACE_1, 0.08)))
                    .text_color(rgb(ACCENT_SECONDARY))
                    .px(px(6.0))
                    .py(px(2.0))
                    .child("Active"),
            );
        }

        if offline {
            row = row.opacity(0.55).cursor_not_allowed();
        } else {
            let select_entity = entity.clone();
            let device_id = unit.id.clone();
            row = row.cursor_pointer().on_click(move |_, _, cx| {
                let device_id = device_id.clone();
                select_entity
                    .update(cx, |this, cx| {
                        this.select_fleet_unit = Some(device_id);
                        this.show_printer_menu = false;
                        cx.notify();
                    })
                    .ok();
            });
        }

        list = list.child(row);
    }

    let manager_entity = entity.clone();
    let show_manager = div()
        .id("printer-show-manager")
        .w_full()
        .flex()
        .items_center()
        .gap(px(8.0))
        .rounded(px(ROUNDED_MD))
        .px(px(8.0))
        .py(px(6.0))
        .text_left()
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(ACCENT_SECONDARY))
        .child(icon("chevron-left", 14.0, ACCENT_SECONDARY))
        .child("Show manager")
        .cursor_pointer()
        .on_click(move |_, _, cx| {
            if actions_disabled {
                return;
            }
            manager_entity
                .update(cx, |this, cx| {
                    this.show_printer_menu = false;
                    this.show_settings = true;
                    cx.notify();
                })
                .ok();
        });

    menu_surface(280.0)
        .absolute()
        .top(px(49.0))
        .left(px(69.0))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(menu_title("FLEET UNITS"))
        .child(list)
        .child(
            div()
                .mt(px(4.0))
                .pt(px(4.0))
                .border_t_1()
                .border_color(rgb(BORDER_SUBTLE))
                .child(show_manager),
        )
}

#[derive(Clone, Copy)]
enum CaptionAction {
    Min,
    Max,
    Close,
}

fn toggle_maximize(window: &mut gpui::Window) {
    if window.is_maximized() {
        restore_window(window);
    } else {
        window.zoom_window();
    }
}

#[cfg(windows)]
fn restore_window(window: &gpui::Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = raw_window_handle::HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return;
    };
    unsafe extern "system" {
        fn ShowWindow(hwnd: isize, cmd: i32) -> i32;
    }
    const SW_RESTORE: i32 = 9;
    unsafe {
        ShowWindow(win32.hwnd.get(), SW_RESTORE);
    }
}

#[cfg(not(windows))]
fn restore_window(window: &gpui::Window) {
    window.zoom_window();
}

/// Starts the native window move. GPUI's `start_window_move` is a no-op on
/// Windows, so this sends the title-bar drag message the OS understands.
#[cfg(windows)]
fn start_window_drag(window: &gpui::Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = raw_window_handle::HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return;
    };
    unsafe extern "system" {
        fn ReleaseCapture() -> i32;
        fn SendMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize;
    }
    const WM_NCLBUTTONDOWN: u32 = 0x00A1;
    const HTCAPTION: usize = 2;
    unsafe {
        ReleaseCapture();
        SendMessageW(win32.hwnd.get(), WM_NCLBUTTONDOWN, HTCAPTION, 0);
    }
}

#[cfg(not(windows))]
fn start_window_drag(window: &gpui::Window) {
    window.start_window_move();
}

//! `src/features/organicCut/OrganicCutPanel.tsx`.
//!
//! Panel body only: the card shell and its collapse chevron belong to
//! `panels.rs`. The header's right-hand controls (the preview eye and the
//! animated reset) move to the top of the body, right-aligned, the same
//! treatment `panel_content.rs` gives the Models add button.
//!
//! # PORT DEVIATIONS
//!
//! * GPUI has no text entry: every `CompactNumberField` carries the port's two
//!   12px chevrons (`panel_fields::stepped_field`); the source types into the
//!   field instead.
//! * `filter: grayscale(0.7)` on the disabled panel has no GPUI equivalent;
//!   `opacity: 0.45` is kept and the desaturation is dropped.
//! * The reset button's one-shot 360° spin (`animate-spin-once`) and the
//!   `transition-colors` timings have no GPUI equivalent; the icon is static and
//!   the colours switch instantly.
//! * `maxHeight: calc(100vh - var(--topbar-height) - 140px)` has no `vh` unit in
//!   GPUI: the body scrolls and the panel stack owns the cap.
//! * `tracking-wide`, `leading-snug` and `line-clamp` on the tenon alert are
//!   approximated with explicit line heights; GPUI's `line_clamp` is used where
//!   the text can wrap.
//! * `title` tooltips are not ported (the panel stack owns the tooltip layer);
//!   the wording is recorded on each control.
//! * `uppercase` is applied to the strings.
//! * The viewport-side tenon gizmo (`tenonTiltRad`, `tenonRollRad`,
//!   `tenonAnchor`) is scene interaction, not panel UI: the panel reads those
//!   fields and only offers the recentre control the source shows.

use std::rc::Rc;

use gpui::{Div, FontWeight, IntoElement, SharedString, div, prelude::*, px, rgb, rgba};

use gpui_core::icons::icon;
use gpui_ui::panel_fields::*;
use gpui_ui::ui::section_label;
use gpui_core::theme::*;

/// `OrganicCutMode`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrganicCutMode {
    Contour,
    Plane,
}

/// `'frustum' | 'dome'`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TenonShape {
    Frustum,
    Dome,
}

/// `TENON_DIM_MAX_MM`.
pub const TENON_DIM_MAX_MM: f32 = 20.0;
/// `FRUSTUM_MIN_WIDTH_MM`.
pub const FRUSTUM_MIN_WIDTH_MM: f32 = 1.0;
/// `DOME_MIN_WIDTH_MM`.
pub const DOME_MIN_WIDTH_MM: f32 = 1.5;
/// `TENON_TOP_SCALE`.
const TENON_TOP_SCALE: f32 = 0.5;
/// `TENON_BASE_OVERLAP_MM`.
const TENON_BASE_OVERLAP_MM: f32 = 0.3;
/// `TENON_FILLET_STEP_MM`.
const TENON_FILLET_STEP_MM: f32 = 0.1;
/// `TENON_TOLERANCE_MAX_MM`.
pub const TENON_TOLERANCE_MAX_MM: f32 = 1.0;

/// `OrganicCutPanelState`.
#[derive(Clone, Debug, PartialEq)]
pub struct OrganicCutState {
    pub cut_mode: OrganicCutMode,
    pub joint_clearance_mm: f32,
    pub smoothing: f32,
    pub membrane_smoothing: f32,
    pub density: f32,
    pub generate_tenon: bool,
    pub tenon_width_mm: f32,
    pub tenon_depth_mm: f32,
    pub tenon_shape: TenonShape,
    pub tenon_fillet_mm: f32,
    pub tenon_tolerance_mm: f32,
    pub tenon_anchor: Option<[f32; 3]>,
    pub tenon_uniform_scale: bool,
    pub tenon_swap_sides: bool,
    pub tenon_tilt_rad: f32,
    pub tenon_roll_rad: f32,
    pub show_preview: bool,
}

impl Default for OrganicCutState {
    /// `DEFAULT_PANEL_STATE`.
    fn default() -> Self {
        Self {
            cut_mode: OrganicCutMode::Contour,
            joint_clearance_mm: 0.0,
            smoothing: 1.0,
            membrane_smoothing: 1.0,
            density: 4.0,
            generate_tenon: false,
            tenon_width_mm: 2.0,
            tenon_depth_mm: 2.5,
            tenon_shape: TenonShape::Frustum,
            tenon_fillet_mm: 0.2,
            tenon_tolerance_mm: 0.1,
            tenon_anchor: None,
            tenon_uniform_scale: true,
            tenon_swap_sides: false,
            tenon_tilt_rad: 0.0,
            tenon_roll_rad: 0.0,
            show_preview: true,
        }
    }
}

impl OrganicCutState {
    /// `DEFAULT_CUT_SETTINGS`: the cut's own factory settings (mode, clearance,
    /// both smoothings, resolution).
    pub fn default_cut_settings() -> Self {
        let defaults = Self::default();
        Self {
            cut_mode: defaults.cut_mode,
            joint_clearance_mm: defaults.joint_clearance_mm,
            smoothing: defaults.smoothing,
            membrane_smoothing: defaults.membrane_smoothing,
            density: defaults.density,
            ..defaults
        }
    }

    /// `DEFAULT_TENON_SETTINGS`: every tenon field, `generate_tenon` included.
    pub fn default_tenon_settings() -> Self {
        Self::default()
    }

    /// `tenonSettingsDirty`: every tenon field except `generateTenon` differs.
    pub fn tenon_settings_dirty(&self) -> bool {
        let defaults = Self::default();
        self.tenon_width_mm != defaults.tenon_width_mm
            || self.tenon_depth_mm != defaults.tenon_depth_mm
            || self.tenon_shape != defaults.tenon_shape
            || self.tenon_fillet_mm != defaults.tenon_fillet_mm
            || self.tenon_tolerance_mm != defaults.tenon_tolerance_mm
            || self.tenon_anchor != defaults.tenon_anchor
            || self.tenon_uniform_scale != defaults.tenon_uniform_scale
            || self.tenon_swap_sides != defaults.tenon_swap_sides
            || self.tenon_tilt_rad != defaults.tenon_tilt_rad
            || self.tenon_roll_rad != defaults.tenon_roll_rad
    }

    /// `cutSettingsDirty`.
    pub fn cut_settings_dirty(&self) -> bool {
        let defaults = Self::default();
        self.cut_mode != defaults.cut_mode
            || self.joint_clearance_mm != defaults.joint_clearance_mm
            || self.smoothing != defaults.smoothing
            || self.membrane_smoothing != defaults.membrane_smoothing
            || self.density != defaults.density
    }

    fn tenon_dim_min_mm(&self) -> f32 {
        if self.tenon_shape == TenonShape::Dome {
            DOME_MIN_WIDTH_MM
        } else {
            FRUSTUM_MIN_WIDTH_MM
        }
    }
}

/// One loop chip: `index` + waypoint count.
#[derive(Clone, Copy, Debug)]
pub struct LoopSummary {
    pub index: usize,
    pub point_count: usize,
}

/// Everything `OrganicCutPanel` draws from.
#[derive(Clone, Debug)]
pub struct OrganicCutView {
    pub state: OrganicCutState,
    pub loop_count: usize,
    pub active_loop_index: usize,
    pub loop_summaries: Vec<LoopSummary>,
    pub can_add_loop: bool,
    pub can_remove_loop: bool,
    /// `canSnapToEdges`: geometry present and at least one waypoint.
    pub can_snap_to_edges: bool,
    pub is_applying: bool,
    pub can_apply: bool,
    pub disabled: bool,
    pub tenon_fits: bool,
    pub cut_error: Option<SharedString>,
    pub tenon_detail: String,
}

impl Default for OrganicCutView {
    fn default() -> Self {
        Self {
            state: OrganicCutState::default(),
            loop_count: 1,
            active_loop_index: 0,
            loop_summaries: Vec::new(),
            can_add_loop: false,
            can_remove_loop: false,
            can_snap_to_edges: false,
            is_applying: false,
            can_apply: false,
            disabled: false,
            tenon_fits: true,
            cut_error: None,
            tenon_detail: String::new(),
        }
    }
}

#[derive(Clone)]
pub struct OrganicCutActions {
    /// `onStateChange` — receives the whole next state, as the source's
    /// `setState` builds it.
    pub on_state_change: Rc<dyn Fn(OrganicCutState)>,
    pub on_clear_loop: Rc<dyn Fn()>,
    pub on_snap_to_edges: Rc<dyn Fn()>,
    pub on_select_loop: Rc<dyn Fn(usize)>,
    pub on_add_loop: Rc<dyn Fn()>,
    pub on_remove_loop: Rc<dyn Fn(usize)>,
    pub on_apply: Rc<dyn Fn()>,
}

impl Default for OrganicCutActions {
    fn default() -> Self {
        Self {
            on_state_change: Rc::new(|_| {}),
            on_clear_loop: Rc::new(|| {}),
            on_snap_to_edges: Rc::new(|| {}),
            on_select_loop: Rc::new(|_| {}),
            on_add_loop: Rc::new(|| {}),
            on_remove_loop: Rc::new(|_| {}),
            on_apply: Rc::new(|| {}),
        }
    }
}

/// `clampFloat(value, min, max, decimals)`.
fn clamp_float(value: f32, min: f32, max: f32, decimals: i32) -> f32 {
    let safe = if value.is_finite() { value } else { min };
    let factor = 10f32.powi(decimals);
    let rounded = (safe * factor).round() / factor;
    rounded.clamp(min, max)
}

/// `maxTenonFilletMm`: the largest fillet the frustum can take at this
/// width/depth, rounded down to the field's step.
fn max_tenon_fillet_mm(width_mm: f32, depth_mm: f32) -> f32 {
    let top_half_width = width_mm * TENON_TOP_SCALE * 0.5;
    let height = depth_mm + TENON_BASE_OVERLAP_MM;
    let limit = (top_half_width * 0.999).min(height / 3.0);
    let stepped = (limit / TENON_FILLET_STEP_MM).floor() * TENON_FILLET_STEP_MM;
    let value = ((stepped * 100.0).round() / 100.0).max(0.0);
    value
}

/// `formatTenonFilletTitle`.
fn tenon_fillet_title(max_mm: f32) -> String {
    format!(
        "Rounds the tenon's corners and tip. On this tenon the geometry accepts up to {max_mm}mm — a wider or deeper tenon raises that ceiling."
    )
}

/// `formatLoopTitle`.
fn loop_title(loop_index: usize, points: usize, incomplete: bool, active: bool) -> String {
    let points_label = if points == 1 {
        "1 point".to_string()
    } else {
        format!("{points} points")
    };
    let suffix = if incomplete {
        if active {
            " (needs 3+ to cut) — editing"
        } else {
            " (needs 3+ to cut) — click to edit"
        }
    } else if active {
        " — editing"
    } else {
        " — click to edit"
    };
    format!("Loop {loop_index} — {points_label}{suffix}")
}

/// `cardStyle` / `accentCardStyle`.
fn cut_card() -> Div {
    plain_sub_card()
}

fn cut_accent_card() -> Div {
    accent_sub_card()
}

/// The Split body: the scrollable controls plus the pinned action row.
pub fn organic_cut_body(view: &OrganicCutView, actions: &OrganicCutActions) -> impl IntoElement {
    let state = &view.state;
    let is_contour = state.cut_mode == OrganicCutMode::Contour;
    let busy = view.disabled || view.is_applying;
    let tenon_blocks_cut = state.generate_tenon && !view.tenon_fits;

    let mut body = div().flex().flex_col().w_full();

    // The header's right-hand controls, moved to the top of the body.
    body = body.child(header_controls(view, actions));

    let mut scroll = div()
        .id("organic-cut-scroll")
        .px(px(10.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .flex_1()
        .min_h(px(0.0))
        .overflow_y_scroll();

    // ── Cut mode ────────────────────────────────────────────────────────────
    let to_contour = actions.on_state_change.clone();
    let to_plane = actions.on_state_change.clone();
    let contour_state = state.clone();
    let plane_state = state.clone();
    scroll = scroll.child(
        cut_accent_card()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(section_label("Cut Mode"))
            .child(
                div()
                    .flex()
                    .gap(px(4.0))
                    .child(div().flex_1().child(mode_pill(
                        "organic-cut-mode-contour",
                        "Contour",
                        is_contour,
                        !busy,
                        move |_, _, _| {
                            let mut next = contour_state.clone();
                            next.cut_mode = OrganicCutMode::Contour;
                            to_contour(next);
                        },
                    )))
                    .child(div().flex_1().child(mode_pill(
                        "organic-cut-mode-flat",
                        "Flat",
                        !is_contour,
                        !busy,
                        move |_, _, _| {
                            let mut next = plane_state.clone();
                            next.cut_mode = OrganicCutMode::Plane;
                            to_plane(next);
                        },
                    ))),
            ),
    );

    // ── Cut settings ────────────────────────────────────────────────────────
    let settings = cut_card().flex().flex_col().gap(px(6.0));
    let mut settings_grid = div().flex().flex_col().gap(px(6.0));

    let mut first_row = div().flex().gap(px(8.0));
    if is_contour {
        let change = actions.on_state_change.clone();
        let base = state.clone();
        first_row = first_row.child(div().flex_1().min_w(px(0.0)).child(compact_field(
            "organic-seam-smoothing",
            "Seam Smoothing",
            state.smoothing,
            None,
            0.05,
            0.0,
            2.0,
            busy,
            Rc::new(move |value| {
                let mut next = base.clone();
                next.smoothing = clamp_float(value, 0.0, 2.0, 2);
                change(next);
            }),
        )));
    }
    {
        let change = actions.on_state_change.clone();
        let base = state.clone();
        let clearance = div().min_w(px(0.0)).child(compact_field(
            "organic-joint-clearance",
            "Joint Clearance",
            state.joint_clearance_mm,
            Some(Unit::new("mm", 10.0)),
            0.05,
            0.0,
            1.5,
            busy,
            Rc::new(move |value| {
                let mut next = base.clone();
                next.joint_clearance_mm = clamp_float(value, 0.0, 1.5, 2);
                change(next);
            }),
        ));
        first_row = first_row.child(if is_contour {
            div().flex_1().child(clearance)
        } else {
            // `col-span-2` in flat mode: the only surviving field stretches.
            div().flex_1().child(clearance)
        });
    }
    settings_grid = settings_grid.child(first_row);

    if is_contour {
        let mut second_row = div().flex().gap(px(8.0));
        {
            let change = actions.on_state_change.clone();
            let base = state.clone();
            second_row = second_row.child(div().flex_1().min_w(px(0.0)).child(compact_field(
                "organic-cut-smoothing",
                "Cut Smoothing",
                state.membrane_smoothing,
                None,
                0.05,
                0.0,
                2.0,
                busy,
                Rc::new(move |value| {
                    let mut next = base.clone();
                    next.membrane_smoothing = clamp_float(value, 0.0, 2.0, 2);
                    change(next);
                }),
            )));
        }
        {
            let change = actions.on_state_change.clone();
            let base = state.clone();
            second_row = second_row.child(div().flex_1().min_w(px(0.0)).child(compact_field(
                "organic-cut-resolution",
                "Cut Resolution",
                state.density,
                Some(Unit::new("×", 10.0)),
                0.5,
                1.0,
                4.0,
                busy,
                Rc::new(move |value| {
                    let mut next = base.clone();
                    next.density = clamp_float(value, 1.0, 4.0, 2);
                    change(next);
                }),
            )));
        }
        settings_grid = settings_grid.child(second_row);
    }

    scroll = scroll.child(settings.child(settings_grid));

    // ── Registration tenon ──────────────────────────────────────────────────
    scroll = scroll.child(tenon_card(view, actions, busy));

    // ── Loops (contour only) ────────────────────────────────────────────────
    if is_contour {
        scroll = scroll.child(loops_card(view, actions, busy));
    }

    // ── Snap to edges ───────────────────────────────────────────────────────
    {
        let snap = actions.on_snap_to_edges.clone();
        let enabled = !busy && view.can_snap_to_edges;
        scroll = scroll.child(panel_button(
            "organic-snap-to-edges",
            "Snap to Edges",
            PanelButton::Secondary,
            None,
            enabled,
            move |_, _, _| snap(),
        ));
    }

    body = body.child(scroll);

    // ── Pinned actions ──────────────────────────────────────────────────────
    let mut footer = div()
        .px(px(10.0))
        .pt(px(8.0))
        .pb(px(10.0))
        .flex()
        .flex_col();

    if let Some(error) = &view.cut_error {
        footer = footer.child(
            div()
                .mb(px(8.0))
                .flex()
                .items_start()
                .gap(px(8.0))
                .rounded(px(4.0))
                .border_1()
                .border_color(rgb(mix(0xb3121b, BORDER_SUBTLE, 0.75)))
                .bg(rgb(mix(0xb3121b, SURFACE_1, 0.20)))
                .px(px(10.0))
                .py(px(8.0))
                .text_size(px(11.0))
                .line_height(px(14.0))
                .text_color(rgb(TEXT_STRONG))
                .child(icon("triangle-alert", 14.0, 0xff6b6b))
                .child(div().flex_1().min_w(px(0.0)).child(error.clone())),
        );
    }

    {
        let clear = actions.on_clear_loop.clone();
        let apply = actions.on_apply.clone();
        let can_clear = !busy && view.loop_summaries.iter().any(|loop_| loop_.point_count > 0);
        let can_cut = !busy && view.can_apply && !tenon_blocks_cut;

        footer = footer.child(
            div()
                .flex()
                .gap(px(8.0))
                .child(div().flex_1().child(panel_button(
                    "organic-clear-all",
                    "Clear All",
                    PanelButton::Secondary,
                    None,
                    can_clear,
                    move |_, _, _| clear(),
                )))
                .child(div().flex_1().child(if view.is_applying {
                    panel_button_element(
                        "organic-cut",
                        busy_label("Cutting...", 12.0),
                        PanelButton::Accent,
                        None,
                        can_cut,
                        move |_, _, _| apply(),
                    )
                } else {
                    panel_button(
                        "organic-cut",
                        "Cut",
                        PanelButton::Accent,
                        None,
                        can_cut,
                        move |_, _, _| apply(),
                    )
                })),
        );
    }

    body.child(footer)
}

/// The header's preview eye and reset, right-aligned at the top of the body.
fn header_controls(view: &OrganicCutView, actions: &OrganicCutActions) -> Div {
    let state = &view.state;
    let busy = view.disabled || view.is_applying;
    let toggle_preview = actions.on_state_change.clone();
    let reset_cut = actions.on_state_change.clone();
    let preview_state = state.clone();
    let reset_state = state.clone();

    let eye = div()
        .id("organic-preview-toggle")
        .p(px(2.0))
        .rounded(px(4.0))
        .child(icon(
            if state.show_preview { "eye" } else { "eye-off" },
            14.0,
            if state.show_preview { ACCENT } else { TEXT_MUTED },
        ));
    let eye = if busy {
        eye.opacity(0.4).cursor_not_allowed()
    } else {
        eye.cursor_pointer()
            .hover(|s| s.bg(rgba(with_alpha(WHITE, 0x1a))))
            .on_click(move |_, _, _| {
                let mut next = preview_state.clone();
                next.show_preview = !next.show_preview;
                toggle_preview(next);
            })
    };

    // `AnimatedResetButton`: `!text-red-400/70 hover:!text-red-400
    // hover:!bg-red-600/15 disabled:opacity-40`.
    let reset_enabled = !busy && state.cut_settings_dirty();
    let reset = div()
        .id("organic-reset-cut")
        .p(px(2.0))
        .rounded(px(4.0))
        .child(icon("rotate-ccw", 14.0, mix(AXIS_X, SURFACE_0, 0.70)));
    let reset = if reset_enabled {
        reset
            .cursor_pointer()
            .hover(|s| {
                s.bg(rgba(with_alpha(0xdc2626, 0x26)))
                    .text_color(rgb(AXIS_X))
            })
            .on_click(move |_, _, _| {
                let mut next = reset_state.clone();
                let defaults = OrganicCutState::default_cut_settings();
                next.cut_mode = defaults.cut_mode;
                next.joint_clearance_mm = defaults.joint_clearance_mm;
                next.smoothing = defaults.smoothing;
                next.membrane_smoothing = defaults.membrane_smoothing;
                next.density = defaults.density;
                reset_cut(next);
            })
    } else {
        reset.opacity(0.4).cursor_not_allowed()
    };

    div()
        .flex()
        .justify_end()
        .gap(px(4.0))
        .px(px(10.0))
        .pt(px(8.0))
        .child(eye)
        .child(reset)
}

/// The registration-tenon card: No Tenon / Frustum / Dome, the size fields, the
/// flip/reset row and the won't-fit alert.
fn tenon_card(view: &OrganicCutView, actions: &OrganicCutActions, busy: bool) -> Div {
    let state = &view.state;
    let tenon_dim_min = state.tenon_dim_min_mm();
    let fillet_max = max_tenon_fillet_mm(state.tenon_width_mm, state.tenon_depth_mm);
    let is_dome = state.tenon_shape == TenonShape::Dome;

    let mut card = cut_card().flex().flex_col().gap(px(6.0));

    // No Tenon / Frustum / Dome.
    let mut shapes = div().flex().gap(px(4.0));
    {
        let change = actions.on_state_change.clone();
        let base = state.clone();
        shapes = shapes.child(div().flex_1().child(mode_pill(
            "organic-tenon-none",
            "No Tenon",
            !state.generate_tenon,
            !busy,
            move |_, _, _| {
                let mut next = base.clone();
                next.generate_tenon = false;
                change(next);
            },
        )));
    }
    {
        let change = actions.on_state_change.clone();
        let base = state.clone();
        shapes = shapes.child(div().flex_1().child(mode_pill(
            "organic-tenon-frustum",
            "Frustum",
            state.generate_tenon && !is_dome,
            !busy && view.can_apply,
            move |_, _, _| {
                let mut next = base.clone();
                next.generate_tenon = true;
                next.tenon_shape = TenonShape::Frustum;
                // Turning a tenon on brings the preview back.
                next.show_preview = true;
                next.tenon_fillet_mm = next
                    .tenon_fillet_mm
                    .min(max_tenon_fillet_mm(next.tenon_width_mm, next.tenon_depth_mm));
                change(next);
            },
        )));
    }
    {
        let change = actions.on_state_change.clone();
        let base = state.clone();
        shapes = shapes.child(div().flex_1().child(mode_pill(
            "organic-tenon-dome",
            "Dome",
            state.generate_tenon && is_dome,
            !busy && view.can_apply,
            move |_, _, _| {
                let mut next = base.clone();
                next.generate_tenon = true;
                next.tenon_shape = TenonShape::Dome;
                next.show_preview = true;
                next.tenon_width_mm = next.tenon_width_mm.max(DOME_MIN_WIDTH_MM);
                next.tenon_depth_mm = next.tenon_depth_mm.max(DOME_MIN_WIDTH_MM);
                change(next);
            },
        )));
    }
    card = card.child(shapes);

    if state.generate_tenon {
        let mut details = div().flex().flex_col().gap(px(6.0)).pt(px(2.0));

        // Recentre — only shown once the handle has moved the tenon.
        if state.tenon_anchor.is_some() {
            let change = actions.on_state_change.clone();
            let base = state.clone();
            details = details.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(FONT_SM))
                            .text_color(rgb(TEXT_MUTED))
                            .child("Moved off centre"),
                    )
                    .child(
                        div()
                            .id("organic-tenon-center")
                            .h(px(24.0))
                            .px(px(11.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(RADIUS_SM))
                            .border_1()
                            .border_color(rgb(BORDER_SUBTLE))
                            .bg(rgb(SURFACE_1))
                            .text_size(px(FONT_MD))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT_STRONG))
                            .cursor_pointer()
                            .when(!busy, |this| {
                                this.hover(|s| s.bg(rgb(SURFACE_2))).on_click(move |_, _, _| {
                                    let mut next = base.clone();
                                    next.tenon_anchor = None;
                                    change(next);
                                })
                            })
                            .child("Center"),
                    ),
            );
        }

        // Width / (link) / Depth.
        let mut dims = div().flex().gap(px(8.0));
        {
            let change = actions.on_state_change.clone();
            let base = state.clone();
            dims = dims.child(div().flex_1().min_w(px(0.0)).child(compact_field(
                "organic-tenon-width",
                "Tenon Width",
                state.tenon_width_mm,
                Some(Unit::new("mm", 10.0)),
                0.5,
                tenon_dim_min,
                TENON_DIM_MAX_MM,
                busy,
                Rc::new(move |value| {
                    let mut next = base.clone();
                    if next.tenon_shape == TenonShape::Dome {
                        set_dome_dim(&mut next, 0, value);
                    } else {
                        set_frustum_dim(&mut next, 0, value);
                    }
                    change(next);
                }),
            )));
        }
        if is_dome {
            let change = actions.on_state_change.clone();
            let base = state.clone();
            let linked = state.tenon_uniform_scale;
            dims = dims.child(
                div()
                    .w(px(24.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .id("organic-tenon-link")
                            .mt(px(4.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(rgb(if linked { ACCENT } else { TEXT_MUTED }))
                            .cursor_pointer()
                            .on_click(move |_, _, _| {
                                let mut next = base.clone();
                                next.tenon_uniform_scale = !next.tenon_uniform_scale;
                                change(next);
                            })
                            .child(icon(
                                if linked { "link-2" } else { "link-2-off" },
                                18.0,
                                if linked { ACCENT } else { TEXT_MUTED },
                            )),
                    ),
            );
        }
        {
            let change = actions.on_state_change.clone();
            let base = state.clone();
            dims = dims.child(div().flex_1().min_w(px(0.0)).child(compact_field(
                "organic-tenon-depth",
                "Tenon Depth",
                state.tenon_depth_mm,
                Some(Unit::new("mm", 10.0)),
                0.5,
                tenon_dim_min,
                TENON_DIM_MAX_MM,
                busy,
                Rc::new(move |value| {
                    let mut next = base.clone();
                    if next.tenon_shape == TenonShape::Dome {
                        set_dome_dim(&mut next, 1, value);
                    } else {
                        set_frustum_dim(&mut next, 1, value);
                    }
                    change(next);
                }),
            )));
        }
        details = details.child(dims);

        let mut knobs = div().flex().gap(px(8.0));
        if !is_dome {
            let change = actions.on_state_change.clone();
            let base = state.clone();
            knobs = knobs.child(div().flex_1().min_w(px(0.0)).child(compact_field(
                "organic-tenon-fillet",
                "Edge Fillet",
                state.tenon_fillet_mm,
                Some(Unit::new("mm", 10.0)),
                TENON_FILLET_STEP_MM,
                0.0,
                fillet_max,
                busy,
                Rc::new(move |value| {
                    let mut next = base.clone();
                    next.tenon_fillet_mm = clamp_float(value, 0.0, fillet_max, 2);
                    change(next);
                }),
            )));
        }
        {
            let change = actions.on_state_change.clone();
            let base = state.clone();
            let tolerance = compact_field(
                "organic-tenon-tolerance",
                "Fit Tolerance",
                state.tenon_tolerance_mm,
                Some(Unit::new("mm", 10.0)),
                0.05,
                0.0,
                TENON_TOLERANCE_MAX_MM,
                busy,
                Rc::new(move |value| {
                    let mut next = base.clone();
                    next.tenon_tolerance_mm =
                        clamp_float(value, 0.0, TENON_TOLERANCE_MAX_MM, 2);
                    change(next);
                }),
            );
            // `className={state.tenonShape === 'frustum' ? undefined : 'col-span-2'}`
            // — the dome has no fillet, so the tolerance takes the whole row.
            knobs = knobs.child(if is_dome {
                div().flex_1().min_w(px(0.0)).child(tolerance)
            } else {
                div().flex_1().min_w(px(0.0)).child(tolerance)
            });
        }
        details = details.child(knobs);

        // Flip / Reset.
        let mut actions_row = div().flex().gap(px(4.0)).pt(px(4.0));
        {
            let change = actions.on_state_change.clone();
            let base = state.clone();
            let colors = (
                mix(ACCENT_SECONDARY, SURFACE_1, 0.05),
                mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.38),
                mix(ACCENT_SECONDARY, TEXT_STRONG, 0.55),
            );
            actions_row = actions_row.child(div().flex_1().child(panel_button(
                "organic-tenon-flip",
                "Flip Tenon",
                PanelButton::Secondary,
                Some(colors),
                !busy,
                move |_, _, _| {
                    let mut next = base.clone();
                    next.tenon_swap_sides = !next.tenon_swap_sides;
                    change(next);
                },
            )));
        }
        {
            let change = actions.on_state_change.clone();
            let base = state.clone();
            let dirty = state.tenon_settings_dirty();
            let colors = (
                mix(AXIS_X, SURFACE_1, 0.10),
                mix(AXIS_X, BORDER_SUBTLE, 0.45),
                mix(AXIS_X, TEXT_STRONG, 0.70),
            );
            actions_row = actions_row.child(div().flex_1().child(panel_button(
                "organic-tenon-reset",
                "Reset",
                PanelButton::Secondary,
                if dirty { Some(colors) } else { None },
                !busy && dirty,
                move |_, _, _| {
                    let mut next = OrganicCutState::default_tenon_settings();
                    next.generate_tenon = base.generate_tenon;
                    change(next);
                },
            )));
        }
        details = details.child(actions_row);

        card = card.child(details);
    }

    if state.generate_tenon && !view.tenon_fits && !view.tenon_detail.is_empty() {
        card = card.child(
            div()
                .rounded(px(4.0))
                .border_1()
                .border_color(rgb(mix(0xb3121b, BORDER_SUBTLE, 0.60)))
                .bg(rgb(mix(0xb3121b, SURFACE_1, 0.12)))
                .px(px(8.0))
                .py(px(6.0))
                .text_size(px(10.0))
                .line_height(px(13.0))
                .text_color(rgb(TEXT_STRONG))
                .child(view.tenon_detail.clone()),
        );
    }

    card
}

/// `setFrustumDim`: clamp the axis, then bring the fillet back under the new
/// ceiling.
fn set_frustum_dim(state: &mut OrganicCutState, axis: usize, value: f32) {
    let min = state.tenon_dim_min_mm();
    let clamped = clamp_float(value, min, TENON_DIM_MAX_MM, 1);
    if axis == 0 {
        state.tenon_width_mm = clamped;
    } else {
        state.tenon_depth_mm = clamped;
    }
    state.tenon_fillet_mm = state.tenon_fillet_mm.min(max_tenon_fillet_mm(
        state.tenon_width_mm,
        state.tenon_depth_mm,
    ));
}

/// `setDomeDim`: with Uniform Scale on, clamp the *factor* so both axes stop
/// together and the current width:depth ratio survives.
fn set_dome_dim(state: &mut OrganicCutState, axis: usize, value: f32) {
    let min = state.tenon_dim_min_mm();
    let clamped = clamp_float(value, min, TENON_DIM_MAX_MM, 1);
    if !state.tenon_uniform_scale {
        if axis == 0 {
            state.tenon_width_mm = clamped;
        } else {
            state.tenon_depth_mm = clamped;
        }
        return;
    }
    let current = if axis == 0 {
        state.tenon_width_mm
    } else {
        state.tenon_depth_mm
    };
    if current <= 0.0 {
        state.tenon_width_mm = clamped;
        state.tenon_depth_mm = clamped;
        return;
    }
    let other = if axis == 0 {
        state.tenon_depth_mm
    } else {
        state.tenon_width_mm
    };
    let mut factor = clamped / current;
    if other > 0.0 {
        factor = factor.min(TENON_DIM_MAX_MM / other);
        factor = factor.max(min / other);
    }
    let next_driven = clamp_float(current * factor, min, TENON_DIM_MAX_MM, 1);
    let next_other = clamp_float(other * factor, min, TENON_DIM_MAX_MM, 1);
    if axis == 0 {
        state.tenon_width_mm = next_driven;
        state.tenon_depth_mm = next_other;
    } else {
        state.tenon_depth_mm = next_driven;
        state.tenon_width_mm = next_other;
    }
}

/// The contour-only loop list: the count, the remove button, one chip per loop
/// and the add button.
fn loops_card(view: &OrganicCutView, actions: &OrganicCutActions, busy: bool) -> Div {
    let count_label = if view.loop_count > 1 {
        format!("Loops ({})", view.loop_count)
    } else {
        "Loops".to_string()
    };

    let remove = actions.on_remove_loop.clone();
    let active = view.active_loop_index;
    let remove_enabled = !busy && view.can_remove_loop;
    let remove_button = div()
        .id("organic-loop-remove")
        .w(px(20.0))
        .h(px(20.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .text_color(rgb(TEXT_MUTED))
        .child(icon("trash-2", 12.0, TEXT_MUTED));
    let remove_button = if remove_enabled {
        remove_button
            .cursor_pointer()
            .hover(|s| s.bg(rgb(SURFACE_2)))
            .on_click(move |_, _, _| remove(active))
    } else {
        remove_button.opacity(0.4).cursor_not_allowed()
    };

    let mut chips = div().flex().flex_wrap().items_center().gap(px(4.0));
    for summary in &view.loop_summaries {
        let select = actions.on_select_loop.clone();
        let is_active = summary.index == view.active_loop_index;
        let incomplete = summary.point_count < 3;
        let index = summary.index;
        let (border, background, text) = if is_active {
            active_mode_style()
        } else if incomplete {
            (BORDER_SUBTLE, SURFACE_1, TEXT_MUTED)
        } else {
            (BORDER_SUBTLE, SURFACE_1, TEXT_STRONG)
        };
        let chip = div()
            .id(gpui::ElementId::named_usize("organic-loop-chip", index))
            .h(px(28.0))
            .min_w(px(28.0))
            .px(px(11.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(RADIUS_SM))
            .border_1()
            .border_color(rgb(border))
            .bg(rgb(background))
            .text_size(px(FONT_MD))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(text))
            .child(format!("{}", index + 1));
        let chip = if busy {
            chip.opacity(0.6).cursor_not_allowed()
        } else {
            chip.cursor_pointer()
                .when(incomplete && !is_active, |this| this.border_dashed())
                .on_click(move |_, _, _| select(index))
        };
        chips = chips.child(chip);
    }

    let add = actions.on_add_loop.clone();
    let add_enabled = !busy && view.can_add_loop;
    let add_chip = div()
        .id("organic-loop-add")
        .h(px(28.0))
        .min_w(px(28.0))
        .px(px(11.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .text_size(px(FONT_MD))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_STRONG))
        .child("+");
    let add_chip = if add_enabled {
        add_chip
            .cursor_pointer()
            .hover(|s| s.bg(rgb(SURFACE_2)))
            .on_click(move |_, _, _| add())
    } else {
        add_chip.opacity(0.6).cursor_not_allowed()
    };
    chips = chips.child(add_chip);

    cut_card()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .child(
                    div()
                        .text_size(px(FONT_SM))
                        .text_color(rgb(TEXT_MUTED))
                        .child(count_label),
                )
                .child(remove_button),
        )
        .child(chips)
}

/// A `Button variant="secondary" size="auto"` mode pill with `!h-8`: the active
/// fill from `activeModeStyle`, the disabled fill from `disabledModeStyle`.
fn mode_pill(
    id: &'static str,
    label: &'static str,
    active: bool,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> gpui::Stateful<Div> {
    let style = active.then(active_mode_style);
    panel_button(id, label, PanelButton::Secondary, style, enabled, on_click)
}

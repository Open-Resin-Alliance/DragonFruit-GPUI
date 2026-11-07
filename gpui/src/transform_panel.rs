//! `src/components/controls/TransformControls.tsx` and
//! `src/components/controls/AutoRotationPanel.tsx`.
//!
//! Both are panel *bodies*: the card, its collapse chevron and the drag handle
//! belong to the panel stack (`panels.rs`), exactly as `panel_content.rs` treats
//! the Models/Supports/Export bodies. The header's right-hand control is kept,
//! moved to the top of the body where the header slot would have put it.
//!
//! # PORT DEVIATIONS
//!
//! * GPUI has no text entry in this slice, so every number field carries the two
//!   12px chevrons from [`crate::panel_fields::stepped_field`]. The source hides
//!   the stepper (`showStepper={false}`) and types into the field instead; the
//!   unit moves 12px left to clear the added gutter.
//! * `max-h-[calc(100vh-180px)]` has no `vh` unit in GPUI: the body is a
//!   scrolling column and the panel stack owns the height cap.
//! * `transition-colors` (140ms) has no GPUI equivalent; colours switch
//!   instantly.
//! * `tracking-wide` (letter-spacing) is omitted — GPUI has no such property.
//! * Auto Orientation's sweep is scene work, not panel UI
//!   (`suggestOrientationForGeometry`, the world-frame bake, the history apply and
//!   the busy modal). This port renders the panel and reports the intent through
//!   `on_orient`; `busy` and `error` are caller-owned, as in the source.
//! * The support-blocker mask store (`getSupportBlockedCount`,
//!   `setSupportBlockerBrushSizeMm`, the `useSyncExternalStore` subscription) is a
//!   scene service: the panel reads `blocked_count` / `brush_size_mm` from its
//!   state and reports changes through callbacks.
//! * The blocker brush's `<input type="range" class="ui-range">` renders as the
//!   `.ui-range` track with the accent fill and a knob at the current value;
//!   dragging is not wired (GPUI has no range element here), matching
//!   `settings.rs`.
//! * `Select` (the native `<select>`) is [`crate::panel_fields::select_field`]:
//!   same box, an in-app menu instead of the OS menu.

use std::f32::consts::PI;
use std::rc::Rc;

use gpui::{Div, FontWeight, IntoElement, SharedString, div, prelude::*, px, rgb};

use crate::panel_fields::*;
use crate::theme::*;

/// `SelectionRotationMode` in `src/features/scene/selectionTransform.ts`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SelectionRotationMode {
    /// Turn the selection as one block around its centre.
    Group,
    /// Turn each model in place.
    Individual,
}

/// Everything `TransformControls` draws from.
#[derive(Clone, Debug)]
pub struct TransformState {
    /// `transform.position`, in mm.
    pub position: [f32; 3],
    /// `transform.rotation`, in radians (the fields show degrees).
    pub rotation: [f32; 3],
    /// `transform.scale`, 1 = 100%.
    pub scale: [f32; 3],
    /// `modelBBox` size; `[1, 1, 1]` when there is no bounding box, matching the
    /// source's `new THREE.Vector3(1, 1, 1)` fallback.
    pub original_size: [f32; 3],
    /// Whether `scene.geom.bbox` exists. Lift/Drop are disabled without it.
    pub has_bbox: bool,
    /// Whether every selected model stands clear of the plate.
    pub auto_lift: bool,
    /// `transformMgr.liftDistance`.
    pub lift_distance: f32,
    pub uniform_scaling: bool,
    /// One flag for move and rotate: the gizmo's frame.
    pub local_space: bool,
    /// `Some` only while several models are selected.
    pub selection_rotation_mode: Option<SelectionRotationMode>,
}

impl Default for TransformState {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            rotation: [0.0; 3],
            scale: [1.0; 3],
            original_size: [1.0; 3],
            has_bbox: false,
            auto_lift: false,
            lift_distance: 5.0,
            uniform_scaling: false,
            local_space: false,
            selection_rotation_mode: None,
        }
    }
}

/// Every callback `TransformControls` takes.
#[derive(Clone)]
pub struct TransformActions {
    pub on_position_change: Rc<dyn Fn([f32; 3])>,
    pub on_position_commit: Rc<dyn Fn()>,
    pub on_center: Rc<dyn Fn()>,
    pub on_rotation_change: Rc<dyn Fn([f32; 3])>,
    pub on_reset_rotation: Rc<dyn Fn()>,
    pub on_rotation_complete: Rc<dyn Fn()>,
    pub on_scale_change: Rc<dyn Fn([f32; 3])>,
    pub on_reset_scale: Rc<dyn Fn()>,
    pub on_scale_commit: Rc<dyn Fn()>,
    pub on_uniform_scaling_change: Rc<dyn Fn(bool)>,
    pub on_local_space_change: Rc<dyn Fn(bool)>,
    pub on_auto_lift_change: Rc<dyn Fn(bool)>,
    pub on_lift_distance_change: Rc<dyn Fn(f32)>,
    pub on_lift: Rc<dyn Fn()>,
    pub on_drop: Rc<dyn Fn()>,
    pub on_transform_commit: Rc<dyn Fn()>,
    pub on_selection_rotation_mode_change: Rc<dyn Fn(SelectionRotationMode)>,
}

fn noop() -> Rc<dyn Fn()> {
    Rc::new(|| {})
}

impl Default for TransformActions {
    fn default() -> Self {
        Self {
            on_position_change: Rc::new(|_| {}),
            on_position_commit: noop(),
            on_center: noop(),
            on_rotation_change: Rc::new(|_| {}),
            on_reset_rotation: noop(),
            on_rotation_complete: noop(),
            on_scale_change: Rc::new(|_| {}),
            on_reset_scale: noop(),
            on_scale_commit: noop(),
            on_uniform_scaling_change: Rc::new(|_| {}),
            on_local_space_change: Rc::new(|_| {}),
            on_auto_lift_change: Rc::new(|_| {}),
            on_lift_distance_change: Rc::new(|_| {}),
            on_lift: noop(),
            on_drop: noop(),
            on_transform_commit: noop(),
            on_selection_rotation_mode_change: Rc::new(|_| {}),
        }
    }
}

/// `(rad * 180) / Math.PI`.
fn to_degrees(radians: f32) -> f32 {
    radians * 180.0 / PI
}

/// `(deg * Math.PI) / 180`.
fn to_radians(degrees: f32) -> f32 {
    degrees * PI / 180.0
}

/// `wrapRotationDegrees`: `((((deg + 180) % 360) + 360) % 360) - 180`, with `-0`
/// normalised to `0` the way `Object.is(wrapped, -0)` does in the source.
fn wrap_rotation_degrees(degrees: f32) -> f32 {
    if !degrees.is_finite() {
        return 0.0;
    }
    let wrapped = (((degrees + 180.0) % 360.0) + 360.0) % 360.0 - 180.0;
    if wrapped == 0.0 { 0.0 } else { wrapped }
}

/// Replace one axis of a triple.
fn with_axis(mut value: [f32; 3], axis: usize, next: f32) -> [f32; 3] {
    value[axis] = next;
    value
}

/// The Transform body: `px-2 pb-2 space-y-2 sm:px-2.5 sm:pb-2.5`, scrolling.
pub fn transform_body(state: &TransformState, actions: &TransformActions) -> impl IntoElement {
    let mut body = panel_body().id("transform-body").overflow_y_scroll();

    body = body.child(move_section(state, actions));
    body = body.child(lift_section(state, actions));
    body = body.child(rotate_section(state, actions));
    body.child(scale_section(state, actions))
}

/// `moveCardStyle`: `color-mix(#4f8cff, border-subtle 78%)` on
/// `color-mix(#4f8cff, surface-1 94%)`.
fn move_card() -> Div {
    sub_card(
        mix(MOVE_TINT, BORDER_SUBTLE, 0.22),
        mix(MOVE_TINT, SURFACE_1, 0.06),
    )
}

/// `liftCardStyle`: `color-mix(#f59e0b, border-subtle 76%)` on
/// `color-mix(#f59e0b, surface-1 93%)`.
fn lift_card() -> Div {
    sub_card(
        mix(LIFT_TINT, BORDER_SUBTLE, 0.24),
        mix(LIFT_TINT, SURFACE_1, 0.07),
    )
}

/// `rotateCardStyle`: `color-mix(#8f6cff, border-subtle 80%)` on
/// `color-mix(#8f6cff, surface-1 95%)`.
fn rotate_card() -> Div {
    sub_card(
        mix(ROTATE_TINT, BORDER_SUBTLE, 0.20),
        mix(ROTATE_TINT, SURFACE_1, 0.05),
    )
}

/// `scaleCardStyle`: `color-mix(#2eb67d, border-subtle 80%)` on
/// `color-mix(#2eb67d, surface-1 95%)`.
fn scale_card() -> Div {
    sub_card(
        mix(SCALE_TINT, BORDER_SUBTLE, 0.20),
        mix(SCALE_TINT, SURFACE_1, 0.05),
    )
}

/// The shared `flex items-center` header row: spacer, centred section title,
/// trailing control.
fn section_row(title: &str, leading: Option<Div>, trailing: Option<Div>) -> Div {
    let row = div()
        .flex()
        .items_center()
        .child(div().flex_1().when_some(leading, |this, leading| {
            this.flex().justify_start().child(leading)
        }))
        .child(section_header(title));
    row.child(div().flex_1().when_some(trailing, |this, trailing| {
        this.flex().justify_end().child(trailing)
    }))
}

/// `renderLocalSpaceToggle`: one button for move and rotate, since they share the
/// gizmo's frame.
fn local_space_toggle(state: &TransformState, actions: &TransformActions) -> Div {
    let toggle = actions.on_local_space_change.clone();
    let local_space = state.local_space;
    div().child(toggle_button(
        "transform-local-space",
        "Local",
        local_space,
        move |_, _, _| toggle(!local_space),
    ))
}

fn move_section(state: &TransformState, actions: &TransformActions) -> Div {
    let mut grid = div().flex().gap(px(4.0));
    for (axis, color) in [(0usize, AXIS_X), (1, AXIS_Y), (2, AXIS_Z)] {
        let change = actions.on_position_change.clone();
        let commit = actions.on_position_commit.clone();
        let position = state.position;
        let label = ["X", "Y", "Z"][axis];
        grid = grid.child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(axis_label(label, color))
                .child(stepped_field(
                    format!("transform-position-{label}"),
                    position[axis],
                    Some(Unit::new("mm", 11.0)),
                    FieldAlign::Left,
                    1.0,
                    f32::NEG_INFINITY,
                    f32::INFINITY,
                    false,
                    Rc::new(move |next| {
                        change(with_axis(position, axis, next));
                        // The source commits on blur; a stepper press is this
                        // port's equivalent of leaving the field.
                        commit();
                    }),
                )),
        );
    }

    let center = actions.on_center.clone();
    move_card()
        .flex()
        .flex_col()
        .child(section_row(
            "Move",
            None,
            Some(local_space_toggle(state, actions)),
        ))
        .child(
            div()
                .pt(px(6.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(grid)
                .child(panel_button(
                    "transform-center",
                    "Center Selection",
                    PanelButton::Secondary,
                    None,
                    true,
                    move |_, _, _| center(),
                )),
        )
}

fn lift_section(state: &TransformState, actions: &TransformActions) -> Div {
    let auto_lift = state.auto_lift;
    let auto_toggle = actions.on_auto_lift_change.clone();
    let distance = actions.on_lift_distance_change.clone();
    let distance_commit = actions.on_transform_commit.clone();
    let lift = actions.on_lift.clone();
    let drop = actions.on_drop.clone();
    let commit = actions.on_transform_commit.clone();
    let commit_for_lift = commit.clone();
    let commit_for_drop = commit;
    let has_bbox = state.has_bbox;

    // The two action buttons keep their inline style while disabled and only dim
    // (`disabled:opacity-50 disabled:cursor-not-allowed`), unlike `.ui-button:disabled`.
    let action_button = |id: &'static str,
                         label: &'static str,
                         colors: (u32, u32, u32),
                         handler: Rc<dyn Fn()>| {
        let (background, border, text) = colors;
        let el = div()
            .id(id)
            .h(px(32.0))
            .px(px(11.0))
            .pt(px(5.0))
            .pb(px(7.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(RADIUS_SM))
            .border_1()
            .border_color(rgb(border))
            .bg(rgb(background))
            .text_size(px(FONT_MD))
            .font_weight(FontWeight::SEMIBOLD)
            .line_height(px(FONT_MD))
            .text_color(rgb(text))
            .child(label);
        if has_bbox {
            el.cursor_pointer()
                .hover(|s| s.bg(rgb(mix(background, WHITE, 0.92))))
                .on_click(move |_, _, _| handler())
        } else {
            el.opacity(0.5).cursor_not_allowed()
        }
    };

    lift_card()
        .flex()
        .flex_col()
        .child(section_row(
            "Lift",
            None,
            Some(
                div().child(toggle_button(
                    "transform-auto-lift",
                    "Auto",
                    auto_lift,
                    move |_, _, _| auto_toggle(!auto_lift),
                )),
            ),
        ))
        .child(
            div()
                .pt(px(6.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(stepped_field(
                    "transform-lift-distance",
                    state.lift_distance,
                    Some(Unit::new("mm", 10.0)),
                    FieldAlign::Center,
                    1.0,
                    f32::NEG_INFINITY,
                    f32::INFINITY,
                    false,
                    Rc::new(move |next| {
                        distance(next);
                        distance_commit();
                    }),
                ))
                .child(
                    div()
                        .flex()
                        .gap(px(4.0))
                        .child(
                            div().flex_1().child(action_button(
                                "transform-lift",
                                "Lift",
                                (
                                    mix(ACCENT, SURFACE_1, 0.10),
                                    mix(ACCENT, BORDER_SUBTLE, 0.62),
                                    mix(ACCENT, TEXT_STRONG, 0.70),
                                ),
                                Rc::new(move || {
                                    lift();
                                    commit_for_lift();
                                }),
                            )),
                        )
                        .child(
                            div().flex_1().child(action_button(
                                "transform-drop",
                                "Drop",
                                (
                                    mix(ACCENT_SECONDARY, SURFACE_1, 0.08),
                                    mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.58),
                                    mix(ACCENT_SECONDARY, TEXT_STRONG, 0.70),
                                ),
                                Rc::new(move || {
                                    drop();
                                    commit_for_drop();
                                }),
                            )),
                        ),
                ),
        )
}

fn rotate_section(state: &TransformState, actions: &TransformActions) -> Div {
    let mut grid = div().flex().gap(px(4.0));
    for (axis, color) in [(0usize, AXIS_X), (1, AXIS_Y), (2, AXIS_Z)] {
        let change = actions.on_rotation_change.clone();
        let complete = actions.on_rotation_complete.clone();
        let commit = actions.on_transform_commit.clone();
        let rotation = state.rotation;
        let label = ["X", "Y", "Z"][axis];
        let degrees = wrap_rotation_degrees(to_degrees(rotation[axis]));
        grid = grid.child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(axis_label(label, color))
                .child(stepped_field(
                    format!("transform-rotation-{label}"),
                    degrees,
                    Some(Unit::new("°", 14.0)),
                    FieldAlign::Left,
                    1.0,
                    f32::NEG_INFINITY,
                    f32::INFINITY,
                    false,
                    Rc::new(move |next| {
                        change(with_axis(
                            rotation,
                            axis,
                            to_radians(wrap_rotation_degrees(next)),
                        ));
                        complete();
                        commit();
                    }),
                )),
        );
    }

    let reset = actions.on_reset_rotation.clone();
    let commit = actions.on_transform_commit.clone();
    let mode = state.selection_rotation_mode;
    let mode_change = actions.on_selection_rotation_mode_change.clone();

    let leading = mode.map(|current| {
        let next = if current == SelectionRotationMode::Group {
            SelectionRotationMode::Individual
        } else {
            SelectionRotationMode::Group
        };
        let change = mode_change.clone();
        div().child(toggle_button(
            "transform-group-rotation",
            "Group",
            current == SelectionRotationMode::Group,
            move |_, _, _| change(next),
        ))
    });

    rotate_card()
        .flex()
        .flex_col()
        .child(section_row(
            "Rotate",
            leading,
            Some(local_space_toggle(state, actions)),
        ))
        .child(
            div()
                .pt(px(6.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(grid)
                .child(panel_button(
                    "transform-reset-rotation",
                    "Reset Rotation",
                    PanelButton::Secondary,
                    None,
                    true,
                    move |_, _, _| {
                        reset();
                        commit();
                    },
                )),
        )
}

fn scale_section(state: &TransformState, actions: &TransformActions) -> Div {
    let uniform = state.uniform_scaling;
    let uniform_toggle = actions.on_uniform_scaling_change.clone();
    let scale = state.scale;
    let original = state.original_size;

    // Percentage row: `scale * 100`, one decimal pair like the source.
    let mut percent_row = div().flex().gap(px(4.0)).items_start();
    for (axis, color) in [(0usize, AXIS_X), (1, AXIS_Y), (2, AXIS_Z)] {
        let change = actions.on_scale_change.clone();
        let commit = actions.on_scale_commit.clone();
        let label = ["X", "Y", "Z"][axis];
        percent_row = percent_row.child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(axis_label(label, color))
                .child(stepped_field(
                    format!("transform-scale-percent-{label}"),
                    scale[axis] * 100.0,
                    Some(Unit::new("%", 10.0)),
                    FieldAlign::Left,
                    1.0,
                    f32::NEG_INFINITY,
                    f32::INFINITY,
                    false,
                    Rc::new(move |next| {
                        let next = next / 100.0;
                        if uniform {
                            change([next, next, next]);
                        } else {
                            change(with_axis(scale, axis, next));
                        }
                        commit();
                    }),
                )),
        );
    }

    // Millimetre row: `scale * originalSize`, divided back on change.
    let mut mm_row = div().flex().gap(px(4.0)).items_start();
    for axis in 0..3 {
        let change = actions.on_scale_change.clone();
        let commit = actions.on_scale_commit.clone();
        let label = ["X", "Y", "Z"][axis];
        let size = original[axis];
        mm_row = mm_row.child(
            div().flex_1().min_w(px(0.0)).child(stepped_field(
                format!("transform-scale-mm-{label}"),
                scale[axis] * size,
                Some(Unit::new("mm", 10.0)),
                FieldAlign::Left,
                1.0,
                f32::NEG_INFINITY,
                f32::INFINITY,
                false,
                Rc::new(move |next| {
                    if size == 0.0 {
                        return;
                    }
                    let next = next / size;
                    if uniform {
                        change([next, next, next]);
                    } else {
                        change(with_axis(scale, axis, next));
                    }
                    commit();
                }),
            )),
        );
    }

    let reset = actions.on_reset_scale.clone();
    let commit = actions.on_scale_commit.clone();

    scale_card()
        .flex()
        .flex_col()
        .child(section_row(
            "Scale",
            None,
            Some(div().child(toggle_button(
                "transform-uniform-scale",
                "Uniform",
                uniform,
                move |_, _, _| uniform_toggle(!uniform),
            ))),
        ))
        .child(
            div()
                .pt(px(6.0))
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(percent_row)
                .child(mm_row)
                .child(panel_button(
                    "transform-reset-scale",
                    "Reset Scale",
                    PanelButton::Secondary,
                    None,
                    true,
                    move |_, _, _| {
                        reset();
                        commit();
                    },
                )),
        )
}

// ---------------------------------------------------------------------------
// Auto Orientation — `AutoRotationPanel.tsx`
// ---------------------------------------------------------------------------

/// `OrientationObjective` in `src/supports/autoSupport/orientationAdvisor.ts`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrientationObjective {
    Supports,
    Height,
    Scarring,
}

impl OrientationObjective {
    fn label(self) -> &'static str {
        match self {
            OrientationObjective::Supports => "Fewest Supports",
            OrientationObjective::Height => "Shortest Print Time",
            OrientationObjective::Scarring => "Least Scarring",
        }
    }

    fn value(self) -> &'static str {
        match self {
            OrientationObjective::Supports => "supports",
            OrientationObjective::Height => "height",
            OrientationObjective::Scarring => "scarring",
        }
    }
}

/// Everything `AutoRotationPanel` draws from.
#[derive(Clone, Debug)]
pub struct OrientationState {
    pub active_model_id: Option<SharedString>,
    pub active_model_name: Option<SharedString>,
    pub objective: OrientationObjective,
    /// Whether the scene is in support-blocker paint mode.
    pub blockers_active: bool,
    /// `getSupportBlockerBrushSizeMm()`.
    pub brush_size_mm: f32,
    /// `getSupportBlockedCount(activeModelId)`.
    pub blocked_count: usize,
    pub busy: bool,
    pub error: Option<SharedString>,
    /// Whether the objective menu is open (GPUI has no native select).
    pub objective_menu_open: bool,
    /// `onToggleBlockers` present. The source disables the Blockers button
    /// without it.
    pub has_toggle_blockers: bool,
}

impl Default for OrientationState {
    fn default() -> Self {
        Self {
            active_model_id: None,
            active_model_name: None,
            objective: OrientationObjective::Supports,
            blockers_active: false,
            brush_size_mm: 2.5,
            blocked_count: 0,
            busy: false,
            error: None,
            objective_menu_open: false,
            has_toggle_blockers: true,
        }
    }
}

/// `SUPPORT_BLOCKER_BRUSH_MIN_MM` / `SUPPORT_BLOCKER_BRUSH_MAX_MM`.
pub const SUPPORT_BLOCKER_BRUSH_MIN_MM: f32 = 0.5;
pub const SUPPORT_BLOCKER_BRUSH_MAX_MM: f32 = 10.0;

#[derive(Clone)]
pub struct OrientationActions {
    pub on_orient: Rc<dyn Fn()>,
    pub on_objective_change: Rc<dyn Fn(OrientationObjective)>,
    pub on_objective_menu_open_change: Rc<dyn Fn(bool)>,
    pub on_toggle_blockers: Rc<dyn Fn()>,
    pub on_clear_blockers: Rc<dyn Fn()>,
    pub on_brush_size_change: Rc<dyn Fn(f32)>,
}

impl Default for OrientationActions {
    fn default() -> Self {
        Self {
            on_orient: noop(),
            on_objective_change: Rc::new(|_| {}),
            on_objective_menu_open_change: Rc::new(|_| {}),
            on_toggle_blockers: noop(),
            on_clear_blockers: noop(),
            on_brush_size_change: Rc::new(|_| {}),
        }
    }
}

/// The Auto Orientation body: `px-2.5 pb-3 space-y-2.5`.
pub fn orientation_body(
    state: &OrientationState,
    actions: &OrientationActions,
) -> impl IntoElement {
    let has_model = state.active_model_id.is_some();
    let mut body = panel_body_wide();

    if state.blockers_active {
        // `Press {Clear} to reset all, or {Done} to apply.` — two lines, the two
        // button labels in `--accent-secondary` semibold.
        let action_span = |text: &'static str| {
            div()
                .text_color(rgb(ACCENT_SECONDARY))
                .font_weight(FontWeight::SEMIBOLD)
                .child(text)
        };
        body = body.child(
            div()
                .rounded(px(ROUNDED_MD))
                .border_1()
                .border_color(rgb(mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.58)))
                .bg(rgb(mix(ACCENT_SECONDARY, SURFACE_1, 0.08)))
                .p(px(8.0))
                .min_h(px(72.0))
                .flex()
                .flex_col()
                .gap(px(6.0))
                .text_center()
                .child(
                    div()
                        .text_center()
                        .text_size(px(FONT_SM))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(ACCENT_SECONDARY))
                        .child("Blocker Painting Mode"),
                )
                .child(
                    div()
                        .flex()
                        .items_start()
                        .justify_center()
                        .min_h(px(32.0))
                        .child(
                            div()
                                .text_center()
                                .text_size(px(10.0))
                                .line_height(px(13.0))
                                .text_color(rgb(TEXT_MUTED))
                                .flex()
                                .flex_col()
                                .child("Paint areas to keep support-free. Orientation avoids them.")
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .flex_wrap()
                                        .justify_center()
                                        .child("Press ")
                                        .child(action_span("Clear"))
                                        .child(" to reset all, or ")
                                        .child(action_span("Done"))
                                        .child(" to apply."),
                                ),
                        ),
                ),
        );
    } else {
        let orient = actions.on_orient.clone();
        let toggle = actions.on_toggle_blockers.clone();
        body = body.child(
            div()
                .flex()
                .gap(px(6.0))
                .child(
                    div().flex_1().child(
                        div()
                            .id("orientation-orient")
                            .h(px(32.0))
                            .px(px(11.0))
                            .pt(px(5.0))
                            .pb(px(7.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(RADIUS_SM))
                            .border_1()
                            .border_color(rgb(ACCENT))
                            .bg(rgb(mix(ACCENT, SURFACE_0, 0.14)))
                            .text_size(px(FONT_MD))
                            .font_weight(FontWeight::SEMIBOLD)
                            .line_height(px(FONT_MD))
                            .text_color(rgb(ACCENT))
                            .child(if state.busy { "Analyzing…" } else { "Orient Model" })
                            .when(state.busy || !has_model, |this| {
                                this.opacity(0.5).cursor_not_allowed()
                            })
                            .when(!state.busy && has_model, |this| {
                                this.cursor_pointer().on_click(move |_, _, _| orient())
                            }),
                    ),
                )
                .child(
                    div().flex_1().child(
                        div()
                            .id("orientation-blockers")
                            .h(px(32.0))
                            .px(px(11.0))
                            .pt(px(5.0))
                            .pb(px(7.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(RADIUS_SM))
                            .border_1()
                            .border_color(rgb(BORDER_SUBTLE))
                            .bg(rgb(SURFACE_1))
                            .text_size(px(FONT_MD))
                            .font_weight(FontWeight::SEMIBOLD)
                            .line_height(px(FONT_MD))
                            .text_color(rgb(TEXT_STRONG))
                            .child("Blockers")
                            .when(state.busy || !has_model || !state.has_toggle_blockers, |this| {
                                this.opacity(0.5).cursor_not_allowed()
                            })
                            .when(
                                !state.busy && has_model && state.has_toggle_blockers,
                                |this| {
                                    this.cursor_pointer()
                                        .hover(|s| s.bg(rgb(SURFACE_2)))
                                        .on_click(move |_, _, _| toggle())
                                },
                            ),
                    ),
                ),
        );
    }

    if state.blockers_active {
        let brush = actions.on_brush_size_change.clone();
        let brush_size_mm = state.brush_size_mm;
        let fraction = ((state.brush_size_mm - SUPPORT_BLOCKER_BRUSH_MIN_MM)
            / (SUPPORT_BLOCKER_BRUSH_MAX_MM - SUPPORT_BLOCKER_BRUSH_MIN_MM))
            .clamp(0.0, 1.0);
        body = body.child(
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .text_size(px(FONT_SM))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Brush Size")
                        .child(format!("{:.1} mm", state.brush_size_mm)),
                )
                // `.ui-range`: 8px tall, `--surface-2` track, `--accent` fill.
                .child(
                    div()
                        .id("orientation-brush-range")
                        .relative()
                        .h(px(8.0))
                        .w_full()
                        .rounded(px(9999.0))
                        .bg(rgb(SURFACE_2))
                        .child(
                            div()
                                .h(px(8.0))
                                .w(gpui::relative(fraction))
                                .rounded(px(9999.0))
                                .bg(rgb(ACCENT)),
                        )
                        .child(
                            div()
                                .id("orientation-brush-knob")
                                .absolute()
                                .top(px(-2.0))
                                .left(gpui::relative((fraction - 0.02).max(0.0)))
                                .w(px(12.0))
                                .h(px(12.0))
                                .rounded(px(9999.0))
                                .bg(rgb(ACCENT))
                                .when(!state.busy && has_model, |this| {
                                    this.cursor_pointer().on_click(move |_, _, _| {
                                        // One step per press; dragging is not wired.
                                        brush(
                                            (brush_size_mm + 0.5)
                                                .min(SUPPORT_BLOCKER_BRUSH_MAX_MM),
                                        )
                                    })
                                }),
                        ),
                ),
        );
    } else {
        let open = state.objective_menu_open;
        let toggle_open = actions.on_objective_menu_open_change.clone();
        let select = actions.on_objective_change.clone();
        body = body.child(
            div().flex().flex_col().gap(px(4.0)).child(select_field(
                "orientation-objective",
                state.objective.label(),
                &[
                    ("supports", "Fewest Supports"),
                    ("height", "Shortest Print Time"),
                    ("scarring", "Least Scarring"),
                ],
                open,
                state.busy || !has_model,
                SURFACE_1,
                true,
                Rc::new(move || toggle_open(!open)),
                Rc::new(move |value| {
                    select(match value {
                        "height" => OrientationObjective::Height,
                        "scarring" => OrientationObjective::Scarring,
                        _ => OrientationObjective::Supports,
                    })
                }),
            )),
        );
    }

    if state.blockers_active {
        let clear = actions.on_clear_blockers.clone();
        let toggle = actions.on_toggle_blockers.clone();
        let can_clear = !state.busy && state.blocked_count > 0;
        body = body.child(
            div()
                .flex()
                .gap(px(6.0))
                .child(div().flex_1().child(panel_button(
                    "orientation-clear-blockers",
                    "Clear",
                    PanelButton::Secondary,
                    None,
                    can_clear,
                    move |_, _, _| clear(),
                )))
                .child(div().flex_1().child(panel_button(
                    "orientation-done-blockers",
                    "Done",
                    PanelButton::Accent,
                    None,
                    !state.busy,
                    move |_, _, _| toggle(),
                ))),
        );
    }

    if let Some(error) = &state.error {
        body = body.child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(AXIS_X))
                .child(error.clone()),
        );
    }

    body
}

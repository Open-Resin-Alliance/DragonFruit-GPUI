//! `src/components/controls/ArrangePanel.tsx` and
//! `src/components/controls/DuplicatePanel.tsx`.
//!
//! Panel bodies only: the card, collapse chevron and drag handle belong to
//! `panels.rs`. Each panel's header badge (the model count, the preview count)
//! moves to the top of the body, right-aligned, where the header slot would
//! have put it — the same treatment `panel_content.rs` gives the Models add
//! button.
//!
//! # PORT DEVIATIONS
//!
//! * GPUI has no text entry in this slice: the two `ScrollableNumberField`s keep
//!   their 32px steppers and value box, and the `MiniStepperField` grid keeps its
//!   two 12px chevrons; typing is replaced by the steppers (`settings.rs` does
//!   the same).
//! * `transition-colors` has no GPUI equivalent; colours switch instantly.
//! * `tracking-wide` is omitted (no letter-spacing property).
//! * `title` tooltips are not ported: GPUI's tooltip needs a hover-state view and
//!   the panel stack owns that layer. The wording is recorded on each control.
//! * `Select` (the native `<select>`) is `panel_fields::select_field`: same box,
//!   an in-app menu instead of the OS menu.
//! * `uppercase` is applied to the strings.
//! * The Duplicate card's `opacity-70` + darker border/fill while nothing is
//!   selected applies to the body here; the shell owns the card's own border and
//!   fill.

use std::rc::Rc;

use gpui::{Div, FontWeight, IntoElement, SharedString, div, prelude::*, px, rgb, rgba};

use crate::icons::icon;
use crate::panel_fields::*;
use crate::theme::*;

/// `ArrangeAnchorMode`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArrangeAnchorMode {
    Center,
    FrontLeft,
    FrontRight,
    BackLeft,
    BackRight,
}

impl ArrangeAnchorMode {
    fn label(self) -> &'static str {
        match self {
            ArrangeAnchorMode::Center => "Center",
            ArrangeAnchorMode::FrontLeft => "Front Left",
            ArrangeAnchorMode::FrontRight => "Front Right",
            ArrangeAnchorMode::BackLeft => "Back Left",
            ArrangeAnchorMode::BackRight => "Back Right",
        }
    }

    fn value(self) -> &'static str {
        match self {
            ArrangeAnchorMode::Center => "center",
            ArrangeAnchorMode::FrontLeft => "front_left",
            ArrangeAnchorMode::FrontRight => "front_right",
            ArrangeAnchorMode::BackLeft => "back_left",
            ArrangeAnchorMode::BackRight => "back_right",
        }
    }
}

/// `ArrangeLayoutMode`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArrangeLayoutMode {
    Auto,
    Array,
}

/// `ArrangePrecisionMode`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArrangePrecisionMode {
    Standard,
    HighPrecision,
}

/// `ArrangePlateFillMode`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArrangePlateFillMode {
    Plates,
    Plate,
}

/// Everything `ArrangePanel` draws from.
#[derive(Clone, Debug)]
pub struct ArrangeState {
    pub precision_mode: ArrangePrecisionMode,
    pub fill_mode: ArrangePlateFillMode,
    pub layout_mode: ArrangeLayoutMode,
    pub spacing_mm: f32,
    pub allow_rotate_on_z: bool,
    pub array_count: [i32; 3],
    pub array_gap: [i32; 3],
    pub anchor_mode: ArrangeAnchorMode,
    /// `scene.models.filter(m => m.visible).length`.
    pub model_count: usize,
    /// Visible models that are also selected.
    pub selected_model_count: usize,
    pub is_applying: bool,
    /// `isDuplicateSetupBlockingArrange`.
    pub disable_arrange_actions: bool,
    /// GPUI has no native select menu; the caller owns its open flag.
    pub anchor_menu_open: bool,
}

impl Default for ArrangeState {
    fn default() -> Self {
        Self {
            precision_mode: ArrangePrecisionMode::Standard,
            fill_mode: ArrangePlateFillMode::Plates,
            layout_mode: ArrangeLayoutMode::Auto,
            spacing_mm: 0.5,
            allow_rotate_on_z: false,
            array_count: [1; 3],
            array_gap: [0; 3],
            anchor_mode: ArrangeAnchorMode::Center,
            model_count: 0,
            selected_model_count: 0,
            is_applying: false,
            disable_arrange_actions: false,
            anchor_menu_open: false,
        }
    }
}

#[derive(Clone)]
pub struct ArrangeActions {
    pub on_precision_mode_change: Rc<dyn Fn(ArrangePrecisionMode)>,
    pub on_fill_mode_change: Rc<dyn Fn(ArrangePlateFillMode)>,
    pub on_layout_mode_change: Rc<dyn Fn(ArrangeLayoutMode)>,
    pub on_spacing_change: Rc<dyn Fn(f32)>,
    pub on_allow_rotate_on_z_change: Rc<dyn Fn(bool)>,
    /// `(axis, count)` — axis 0/1/2 is X/Y/Z.
    pub on_array_count_change: Rc<dyn Fn(usize, i32)>,
    /// `(axis, gap)`.
    pub on_array_gap_change: Rc<dyn Fn(usize, i32)>,
    pub on_anchor_mode_change: Rc<dyn Fn(ArrangeAnchorMode)>,
    pub on_anchor_menu_open_change: Rc<dyn Fn(bool)>,
    pub on_apply_all: Rc<dyn Fn()>,
    pub on_apply_selected: Rc<dyn Fn()>,
}

fn noop() -> Rc<dyn Fn()> {
    Rc::new(|| {})
}

impl Default for ArrangeActions {
    fn default() -> Self {
        Self {
            on_precision_mode_change: Rc::new(|_| {}),
            on_fill_mode_change: Rc::new(|_| {}),
            on_layout_mode_change: Rc::new(|_| {}),
            on_spacing_change: Rc::new(|_| {}),
            on_allow_rotate_on_z_change: Rc::new(|_| {}),
            on_array_count_change: Rc::new(|_, _| {}),
            on_array_gap_change: Rc::new(|_, _| {}),
            on_anchor_mode_change: Rc::new(|_| {}),
            on_anchor_menu_open_change: Rc::new(|_| {}),
            on_apply_all: noop(),
            on_apply_selected: noop(),
        }
    }
}

/// `formatModelCount`: `# model` / `# models`.
fn format_model_count(count: usize) -> String {
    if count == 1 {
        "1 model".to_string()
    } else {
        format!("{count} models")
    }
}

/// The header badge both Arrange and Duplicate wear: `inline-flex items-center
/// gap-1 rounded-full border px-2 py-0.5`, a 12px accent glyph and a 10px muted
/// label.
fn header_badge(icon_name: &'static str, label: String) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .rounded(px(9999.0))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .px(px(8.0))
        .py(px(2.0))
        .child(icon(icon_name, 12.0, ACCENT))
        .child(
            div()
                .text_size(px(10.0))
                .text_color(rgb(TEXT_MUTED))
                .child(label),
        )
}

/// One mode pill: `Button variant="secondary" size="auto"` with `!h-8`, the
/// active fill from `activeModeStyle`, and `disabledModeStyle` while applying.
fn mode_button(
    id: &'static str,
    label: &'static str,
    active: bool,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> gpui::Stateful<Div> {
    if !enabled {
        let (background, border, text) = disabled_action_style();
        return div()
            .id(id)
            .w_full()
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
            .cursor_not_allowed()
            .child(label);
    }
    let style = active.then(active_mode_style);
    panel_button(
        id,
        label,
        PanelButton::Secondary,
        style,
        true,
        on_click,
    )
}

/// The two-column mode grid: `grid grid-cols-2 gap-1`. Each cell is a flex-1
/// column so the two `w-full` buttons split the row.
fn mode_grid(buttons: Vec<gpui::Stateful<Div>>) -> Div {
    let mut row = div().flex().gap(px(4.0)).w_full();
    for button in buttons {
        row = row.child(div().flex_1().min_w(px(0.0)).child(button));
    }
    row
}

/// The Arrange body: `px-2 pb-2 space-y-2 sm:px-2.5 sm:pb-2.5`.
pub fn arrange_body(state: &ArrangeState, actions: &ArrangeActions) -> impl IntoElement {
    let is_applying = state.is_applying;
    let is_auto = state.layout_mode == ArrangeLayoutMode::Auto;

    let mut body = panel_body();

    // Header badge (the card header's right slot).
    body = body.child(
        div()
            .flex()
            .justify_end()
            .child(header_badge(
                "layout-grid",
                format_model_count(state.model_count),
            )),
    );

    // Layout Mode.
    let layout_change = actions.on_layout_mode_change.clone();
    body = body.child(
        accent_sub_card()
            .flex()
            .flex_col()
            .child(
                div()
                    .mb(px(4.0))
                    .text_size(px(FONT_SM))
                    .text_color(rgb(TEXT_MUTED))
                    .child("Layout Mode"),
            )
            .child(mode_grid(vec![
                mode_button(
                    "arrange-layout-auto",
                    "Auto",
                    is_auto,
                    !is_applying,
                    move |_, _, _| layout_change(ArrangeLayoutMode::Auto),
                ),
                {
                    let layout_change = actions.on_layout_mode_change.clone();
                    mode_button(
                        "arrange-layout-array",
                        "Manual",
                        state.layout_mode == ArrangeLayoutMode::Array,
                        !is_applying,
                        move |_, _, _| layout_change(ArrangeLayoutMode::Array),
                    )
                },
            ])),
    );

    if is_auto {
        // Plate Mode.
        body = body.child(
            accent_sub_card()
                .flex()
                .flex_col()
                .child(
                    div()
                        .mb(px(4.0))
                        .text_size(px(FONT_SM))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Plate Mode"),
                )
                .child(mode_grid(vec![
                    {
                        let fill_change = actions.on_fill_mode_change.clone();
                        mode_button(
                            "arrange-fill-plates",
                            "Multi-Plate",
                            state.fill_mode == ArrangePlateFillMode::Plates,
                            !is_applying,
                            move |_, _, _| fill_change(ArrangePlateFillMode::Plates),
                        )
                    },
                    {
                        let fill_change = actions.on_fill_mode_change.clone();
                        mode_button(
                            "arrange-fill-plate",
                            "This Plate",
                            state.fill_mode == ArrangePlateFillMode::Plate,
                            !is_applying,
                            move |_, _, _| fill_change(ArrangePlateFillMode::Plate),
                        )
                    },
                ])),
        );

        // Arrange Mode.
        body = body.child(
            accent_sub_card()
                .flex()
                .flex_col()
                .child(
                    div()
                        .mb(px(4.0))
                        .text_size(px(FONT_SM))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Arrange Mode"),
                )
                .child(mode_grid(vec![
                    {
                        let precision = actions.on_precision_mode_change.clone();
                        mode_button(
                            "arrange-precision-standard",
                            "Standard",
                            state.precision_mode == ArrangePrecisionMode::Standard,
                            !is_applying,
                            move |_, _, _| precision(ArrangePrecisionMode::Standard),
                        )
                    },
                    {
                        // `onPrecisionModeChange('high_precision')` also turns
                        // Z-rotation on: the mode requires it.
                        let precision = actions.on_precision_mode_change.clone();
                        let rotate = actions.on_allow_rotate_on_z_change.clone();
                        mode_button(
                            "arrange-precision-high",
                            "High-Precision",
                            state.precision_mode == ArrangePrecisionMode::HighPrecision,
                            !is_applying,
                            move |_, _, _| {
                                precision(ArrangePrecisionMode::HighPrecision);
                                rotate(true);
                            },
                        )
                    },
                ])),
        );
    }

    if is_auto {
        let spacing = actions.on_spacing_change.clone();
        body = body.child(
            plain_sub_card()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(FONT_SM))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Arrange Distance"),
                )
                .child(
                    div().mt(px(4.0)).child(scrollable_field(
                        "arrange-spacing",
                        state.spacing_mm,
                        Some(Unit::new("mm", 10.0)),
                        0.1,
                        -50.0,
                        5.0,
                        is_applying,
                        Rc::new(move |next| {
                            // `setClampedSpacing`: one decimal, clamped to [-50, 5].
                            let rounded = (next * 10.0).round() / 10.0;
                            spacing(rounded.clamp(-50.0, 5.0));
                        }),
                    )),
                ),
        );
    } else {
        body = body.child(array_grid(state, actions));
    }

    // Placement Anchor.
    let anchor_change = actions.on_anchor_mode_change.clone();
    let anchor_open = actions.on_anchor_menu_open_change.clone();
    let anchor_menu_open = state.anchor_menu_open;
    body = body.child(
        plain_sub_card()
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(FONT_SM))
                    .text_color(rgb(TEXT_MUTED))
                    .child("Placement Anchor"),
            )
            .child(div().mt(px(4.0)).child(select_field(
                "arrange-anchor",
                state.anchor_mode.label(),
                &[
                    ("center", "Center"),
                    ("front_left", "Front Left"),
                    ("front_right", "Front Right"),
                    ("back_left", "Back Left"),
                    ("back_right", "Back Right"),
                ],
                state.anchor_menu_open,
                is_applying,
                SURFACE_0,
                false,
                Rc::new(move || anchor_open(!anchor_menu_open)),
                Rc::new(move |value| {
                    anchor_change(match value {
                        "front_left" => ArrangeAnchorMode::FrontLeft,
                        "front_right" => ArrangeAnchorMode::FrontRight,
                        "back_left" => ArrangeAnchorMode::BackLeft,
                        "back_right" => ArrangeAnchorMode::BackRight,
                        _ => ArrangeAnchorMode::Center,
                    })
                }),
            ))),
    );

    if is_auto {
        body = body.child(allow_z_rotation(state, actions));
    }

    // Actions.
    let is_all_disabled =
        state.model_count == 0 || is_applying || state.disable_arrange_actions;
    let is_selected_disabled =
        state.selected_model_count == 0 || is_applying || state.disable_arrange_actions;
    let apply_all = actions.on_apply_all.clone();
    let apply_selected = actions.on_apply_selected.clone();

    let all_variant = if is_all_disabled {
        PanelButton::Secondary
    } else {
        PanelButton::Primary
    };
    let selected_variant = if is_selected_disabled {
        PanelButton::Secondary
    } else {
        PanelButton::Accent
    };

    body.child(
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(if is_applying {
                panel_button_element(
                    "arrange-apply-all",
                    busy_label("Arranging…", 14.0),
                    all_variant,
                    None,
                    !is_all_disabled,
                    move |_, _, _| apply_all(),
                )
            } else {
                panel_button(
                    "arrange-apply-all",
                    "Arrange All",
                    all_variant,
                    None,
                    !is_all_disabled,
                    move |_, _, _| apply_all(),
                )
            })
            .child(if is_applying {
                panel_button_element(
                    "arrange-apply-selected",
                    busy_label("Arranging…", 14.0),
                    selected_variant,
                    None,
                    !is_selected_disabled,
                    move |_, _, _| apply_selected(),
                )
            } else {
                panel_button(
                    "arrange-apply-selected",
                    "Arrange Selected",
                    selected_variant,
                    None,
                    !is_selected_disabled,
                    move |_, _, _| apply_selected(),
                )
            }),
    )
}

/// The Manual-mode array grid: `grid-cols-[24px_minmax(0,1fr)_minmax(0,1fr)]`,
/// the Count/Gap headings, then one row per axis with a 32px chevron field each.
fn array_grid(state: &ArrangeState, actions: &ArrangeActions) -> Div {
    let heading = |label: &'static str| {
        div()
            .flex_1()
            .text_center()
            .text_size(px(10.0))
            .text_color(rgb(TEXT_MUTED))
            .child(label)
    };

    let mut card = plain_sub_card().flex().flex_col();
    card = card.child(
        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .mb(px(4.0))
            .child(div().w(px(24.0)))
            .child(heading("Count"))
            .child(heading("Gap")),
    );

    for (axis, label) in ["X", "Y", "Z"].iter().enumerate() {
        let count = actions.on_array_count_change.clone();
        let gap = actions.on_array_gap_change.clone();
        card = card.child(
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .mb(px(4.0))
                .child(
                    div()
                        .w(px(24.0))
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_MUTED))
                        .child(format!("{label}:")),
                )
                .child(
                    div().flex_1().min_w(px(0.0)).child(chevron_field(
                        format!("arrange-count-{label}"),
                        state.array_count[axis],
                        1,
                        64,
                        state.is_applying,
                        Rc::new(move |next| count(axis, next.clamp(1, 64))),
                    )),
                )
                .child(
                    div().flex_1().min_w(px(0.0)).child(chevron_field(
                        format!("arrange-gap-{label}"),
                        state.array_gap[axis],
                        -120,
                        120,
                        state.is_applying,
                        Rc::new(move |next| gap(axis, next.clamp(-120, 120))),
                    )),
                ),
        );
    }
    card
}

/// The full-width `Allow Z-rotation` toggle row.
fn allow_z_rotation(state: &ArrangeState, actions: &ArrangeActions) -> gpui::Stateful<Div> {
    let high_precision = state.precision_mode == ArrangePrecisionMode::HighPrecision;
    let disabled = state.is_applying || high_precision;
    let toggle = actions.on_allow_rotate_on_z_change.clone();
    let on = state.allow_rotate_on_z;

    let (border, background) = if on {
        (
            mix(ACCENT, BORDER_SUBTLE, 0.24),
            mix(ACCENT, SURFACE_1, 0.05),
        )
    } else {
        (BORDER_SUBTLE, SURFACE_1)
    };

    let pill = div()
        .rounded(px(9999.0))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .px(px(8.0))
        .py(px(2.0))
        .text_size(px(10.0))
        .text_color(rgb(if on { ACCENT } else { TEXT_MUTED }))
        .when(on, |this| {
            this.bg(rgba(with_alpha(ACCENT, 0x1f)))
        })
        .child(if high_precision {
            "ON (Required)"
        } else if on {
            "ON"
        } else {
            "OFF"
        });

    let row = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(icon(
                    "rotate-cw",
                    14.0,
                    if high_precision { TEXT_MUTED } else { ACCENT },
                ))
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Allow Z-rotation"),
                ),
        )
        .child(pill);

    let el = div()
        .id("arrange-allow-z")
        .w_full()
        .rounded(px(RADIUS_MD))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .px(px(8.0))
        .py(px(8.0))
        .child(row);

    if disabled {
        el.opacity(0.6).cursor_not_allowed()
    } else {
        el.cursor_pointer()
            .on_click(move |_, _, _| toggle(!on))
    }
}

// ---------------------------------------------------------------------------
// Duplicate — `DuplicatePanel.tsx`
// ---------------------------------------------------------------------------

/// `DuplicateLayoutMode`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DuplicateLayoutMode {
    Auto,
    Array,
}

/// Everything `DuplicatePanel` draws from.
#[derive(Clone, Debug)]
pub struct DuplicateState {
    pub active_model_name: Option<SharedString>,
    pub layout_mode: DuplicateLayoutMode,
    pub precision_mode: ArrangePrecisionMode,
    pub total_copies: f32,
    pub spacing_mm: f32,
    pub array_count: [i32; 3],
    pub array_gap: [i32; 3],
    /// `duplicatePreviewTransforms.length`.
    pub preview_count: usize,
    pub is_applying: bool,
}

impl Default for DuplicateState {
    fn default() -> Self {
        Self {
            active_model_name: None,
            layout_mode: DuplicateLayoutMode::Auto,
            precision_mode: ArrangePrecisionMode::Standard,
            total_copies: 1.0,
            spacing_mm: 0.5,
            array_count: [1; 3],
            array_gap: [0; 3],
            preview_count: 0,
            is_applying: false,
        }
    }
}

#[derive(Clone)]
pub struct DuplicateActions {
    pub on_layout_mode_change: Rc<dyn Fn(DuplicateLayoutMode)>,
    pub on_precision_mode_change: Rc<dyn Fn(ArrangePrecisionMode)>,
    pub on_total_copies_change: Rc<dyn Fn(f32)>,
    pub on_spacing_change: Rc<dyn Fn(f32)>,
    pub on_array_count_change: Rc<dyn Fn(usize, i32)>,
    pub on_array_gap_change: Rc<dyn Fn(usize, i32)>,
    pub on_confirm: Rc<dyn Fn()>,
    pub on_fill_plate: Rc<dyn Fn()>,
}

impl Default for DuplicateActions {
    fn default() -> Self {
        Self {
            on_layout_mode_change: Rc::new(|_| {}),
            on_precision_mode_change: Rc::new(|_| {}),
            on_total_copies_change: Rc::new(|_| {}),
            on_spacing_change: Rc::new(|_| {}),
            on_array_count_change: Rc::new(|_, _| {}),
            on_array_gap_change: Rc::new(|_, _| {}),
            on_confirm: noop(),
            on_fill_plate: noop(),
        }
    }
}

/// `panelCardStyleDisabled`: `color-mix(border-subtle, black 10%)` on
/// `color-mix(surface-1, black 8%)`.
fn disabled_card() -> Div {
    sub_card(
        mix(BORDER_SUBTLE, BLACK, 0.90),
        mix(SURFACE_1, BLACK, 0.92),
    )
}

/// `accentCardStyleDisabled`: the same border on
/// `color-mix(surface-1, black 6%)`.
fn disabled_accent_card() -> Div {
    sub_card(
        mix(BORDER_SUBTLE, BLACK, 0.90),
        mix(SURFACE_1, BLACK, 0.94),
    )
}

/// The Duplicate body: `px-2 pb-2 space-y-2 sm:px-2.5 sm:pb-2.5`.
pub fn duplicate_body(state: &DuplicateState, actions: &DuplicateActions) -> impl IntoElement {
    let has_selection = state.active_model_name.is_some();
    let panel_disabled = !has_selection || state.is_applying;
    let is_auto = state.layout_mode == DuplicateLayoutMode::Auto;
    let is_high_precision_fill =
        is_auto && state.precision_mode == ArrangePrecisionMode::HighPrecision;

    let display_total_copies = if is_auto {
        state.total_copies.max(1.0).round()
    } else {
        (state.array_count[0].max(1) as f32
            * state.array_count[1].max(1) as f32
            * state.array_count[2].max(1) as f32)
            .max(1.0)
    };

    let mut body = panel_body();
    body = body.child(
        div()
            .flex()
            .justify_end()
            .child(header_badge(
                "copy-plus",
                if is_high_precision_fill {
                    "SAT fill".to_string()
                } else {
                    format!("+{} preview", state.preview_count)
                },
            )),
    );

    // The active model's name, or the empty-state wording.
    body = body.child(
        if panel_disabled {
            disabled_card()
        } else {
            plain_sub_card()
        }
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .w_full()
                .truncate()
                .text_center()
                .text_size(px(12.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(TEXT_STRONG))
                .child(
                    state
                        .active_model_name
                        .clone()
                        .unwrap_or_else(|| "Select a model first".into()),
                ),
        ),
    );

    // Layout Mode.
    let layout_change = actions.on_layout_mode_change.clone();
    body = body.child(
        if panel_disabled {
            disabled_accent_card()
        } else {
            accent_sub_card()
        }
        .flex()
        .flex_col()
        .child(
            div()
                .mb(px(4.0))
                .text_size(px(FONT_SM))
                .text_color(rgb(TEXT_MUTED))
                .child("Layout Mode"),
        )
        .child(mode_grid(vec![
            {
                let layout_change = layout_change.clone();
                mode_button(
                    "duplicate-layout-auto",
                    "Auto Layout",
                    is_auto,
                    !panel_disabled,
                    move |_, _, _| layout_change(DuplicateLayoutMode::Auto),
                )
            },
            {
                let layout_change = layout_change.clone();
                mode_button(
                    "duplicate-layout-array",
                    "Array",
                    state.layout_mode == DuplicateLayoutMode::Array,
                    !panel_disabled,
                    move |_, _, _| layout_change(DuplicateLayoutMode::Array),
                )
            },
        ])),
    );

    if is_auto {
        let precision_change = actions.on_precision_mode_change.clone();
        body = body.child(
            if panel_disabled {
                disabled_accent_card()
            } else {
                accent_sub_card()
            }
            .flex()
            .flex_col()
            .child(
                div()
                    .mb(px(4.0))
                    .text_size(px(FONT_SM))
                    .text_color(rgb(TEXT_MUTED))
                    .child("Precision Mode"),
            )
            .child(mode_grid(vec![
                {
                    let precision_change = precision_change.clone();
                    mode_button(
                        "duplicate-precision-standard",
                        "Standard",
                        state.precision_mode == ArrangePrecisionMode::Standard,
                        !panel_disabled,
                        move |_, _, _| precision_change(ArrangePrecisionMode::Standard),
                    )
                },
                {
                    let precision_change = precision_change.clone();
                    mode_button(
                        "duplicate-precision-high",
                        "High-Precision",
                        state.precision_mode == ArrangePrecisionMode::HighPrecision,
                        !panel_disabled,
                        move |_, _, _| precision_change(ArrangePrecisionMode::HighPrecision),
                    )
                },
            ])),
        );
    }

    if is_auto {
        if state.precision_mode != ArrangePrecisionMode::HighPrecision {
            let copies = actions.on_total_copies_change.clone();
            body = body.child(
                if panel_disabled {
                    disabled_card()
                } else {
                    plain_sub_card()
                }
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(FONT_SM))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Total Copies"),
                )
                .child(
                    div().mt(px(4.0)).child(scrollable_field(
                        "duplicate-copies",
                        state.total_copies,
                        None,
                        1.0,
                        1.0,
                        128.0,
                        panel_disabled,
                        Rc::new(move |next| {
                            let rounded = next.round().clamp(1.0, 128.0);
                            copies(rounded);
                        }),
                    )),
                ),
            );
        }

        let spacing = actions.on_spacing_change.clone();
        body = body.child(
            if panel_disabled {
                disabled_card()
            } else {
                plain_sub_card()
            }
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(FONT_SM))
                    .text_color(rgb(TEXT_MUTED))
                    .child("Arrange Distance"),
            )
            .child(
                div().mt(px(4.0)).child(scrollable_field(
                    "duplicate-spacing",
                    state.spacing_mm,
                    Some(Unit::new("mm", 10.0)),
                    0.1,
                    -5.0,
                    5.0,
                    panel_disabled,
                    Rc::new(move |next| {
                        let rounded = (next * 10.0).round() / 10.0;
                        spacing(rounded.clamp(-5.0, 5.0));
                    }),
                )),
            ),
        );
    } else {
        body = body.child(duplicate_array_grid(state, actions, panel_disabled));
    }

    // Fill Plate.
    let fill_disabled = panel_disabled || !is_auto;
    let fill_plate = actions.on_fill_plate.clone();
    let fill_variant = if fill_disabled {
        PanelButton::Secondary
    } else {
        PanelButton::Accent
    };
    body = body.child(if state.is_applying && is_high_precision_fill {
        panel_button_element(
            "duplicate-fill-plate",
            busy_label("Filling Plate…", 14.0),
            fill_variant,
            None,
            !fill_disabled,
            move |_, _, _| fill_plate(),
        )
    } else {
        panel_button(
            "duplicate-fill-plate",
            "Fill Plate",
            fill_variant,
            None,
            !fill_disabled,
            move |_, _, _| fill_plate(),
        )
    });

    if !is_high_precision_fill {
        let confirm_disabled =
            panel_disabled || state.preview_count == 0 || display_total_copies <= 1.0;
        let confirm = actions.on_confirm.clone();
        let confirm_variant = if confirm_disabled {
            PanelButton::Secondary
        } else {
            PanelButton::Primary
        };
        body = body.child(if state.is_applying {
            panel_button_element(
                "duplicate-confirm",
                busy_label("Duplicating…", 14.0),
                confirm_variant,
                None,
                !confirm_disabled,
                move |_, _, _| confirm(),
            )
        } else {
            panel_button(
                "duplicate-confirm",
                format!("Confirm Duplicate ({} new)", state.preview_count.max(0)),
                confirm_variant,
                None,
                !confirm_disabled,
                move |_, _, _| confirm(),
            )
        });
    }

    if !has_selection {
        // `className={!hasSelection ? 'opacity-70' : undefined}`.
        body = body.opacity(0.7);
    }

    body
}

/// The Duplicate Manual-mode array grid (counts 1..32, gaps -120..120).
fn duplicate_array_grid(
    state: &DuplicateState,
    actions: &DuplicateActions,
    disabled: bool,
) -> Div {
    let heading = |label: &'static str| {
        div()
            .flex_1()
            .text_center()
            .text_size(px(10.0))
            .text_color(rgb(TEXT_MUTED))
            .child(label)
    };

    let mut card = if disabled {
        disabled_card()
    } else {
        plain_sub_card()
    }
    .flex()
    .flex_col();

    card = card.child(
        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .mb(px(4.0))
            .child(div().w(px(24.0)))
            .child(heading("Count"))
            .child(heading("Gap")),
    );

    for (axis, label) in ["X", "Y", "Z"].iter().enumerate() {
        let count = actions.on_array_count_change.clone();
        let gap = actions.on_array_gap_change.clone();
        card = card.child(
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .mb(px(4.0))
                .child(
                    div()
                        .w(px(24.0))
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_MUTED))
                        .child(format!("{label}:")),
                )
                .child(
                    div().flex_1().min_w(px(0.0)).child(chevron_field(
                        format!("duplicate-count-{label}"),
                        state.array_count[axis],
                        1,
                        32,
                        disabled,
                        Rc::new(move |next| count(axis, next.clamp(1, 32))),
                    )),
                )
                .child(
                    div().flex_1().min_w(px(0.0)).child(chevron_field(
                        format!("duplicate-gap-{label}"),
                        state.array_gap[axis],
                        -120,
                        120,
                        disabled,
                        Rc::new(move |next| gap(axis, next.clamp(-120, 120))),
                    )),
                ),
        );
    }
    card
}

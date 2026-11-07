//! `src/features/mesh-smoothing/MeshSmoothingSettingsPanel.tsx`.
//!
//! Panel body only: the card and its collapse chevron belong to `panels.rs`.
//! The settings store (`getMeshSmoothingSettings`, the localStorage round-trip,
//! the subscription) is app state, so the panel reads a [`SmoothingState`] and
//! reports every change through [`SmoothingActions`].
//!
//! # PORT DEVIATIONS
//!
//! * GPUI has no text entry: the three `NumberInput`s carry the port's two 12px
//!   chevrons (`panel_fields::stepped_field`), and the hex field renders as its
//!   value box — the colour is changed through the picker instead.
//! * `react-colorful`'s `HexColorPicker` is a DOM component with a drag surface.
//!   It is replaced by the same 112px rounded container painted with the picker's
//!   two gradients (white → hue horizontally, transparent → black vertically) and
//!   a knob at the current saturation/value. Dragging is not wired, matching the
//!   colour-picker treatment in `settings.rs`; the hex field and the swatch are
//!   live.
//! * `max-h-[calc(100vh - var(--topbar-height) - 88px)]` has no `vh` unit in
//!   GPUI: the body scrolls and the panel stack owns the height cap.
//! * `transition-colors` and `tracking-wide` have no GPUI equivalents.
//! * `uppercase` is applied to the strings.
//! * The source saves to localStorage on every settings change; persistence is
//!   the caller's job here (`on_change` is the whole contract).

use std::rc::Rc;

use gpui::{
    Div, IntoElement, SharedString, div, linear_color_stop, linear_gradient, prelude::*, px, rgb,
    rgba,
};

use gpui_ui::panel_fields::*;
use gpui_ui::ui::{ButtonSize, ButtonSpec, ButtonVariant, button_with, section_header, sub_card_with};
use gpui_core::theme::*;

/// `MeshSmoothingFalloff`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MeshSmoothingFalloff {
    Linear,
    Smooth,
    Sharp,
}

impl MeshSmoothingFalloff {
    /// `FALLOFF_LABELS`, uppercased by the button's class.
    fn label(self) -> &'static str {
        match self {
            MeshSmoothingFalloff::Linear => "Linear",
            MeshSmoothingFalloff::Smooth => "Smooth",
            MeshSmoothingFalloff::Sharp => "Sharp",
        }
    }
}

/// `MESH_SMOOTHING_BRUSH_SIZE_MM`.
pub const MESH_SMOOTHING_BRUSH_MIN_MM: f32 = 0.1;
pub const MESH_SMOOTHING_BRUSH_MAX_MM: f32 = 0.5;
pub const MESH_SMOOTHING_BRUSH_STEP_MM: f32 = 0.01;

/// `DEFAULT_MESH_SMOOTHING_SETTINGS`.
pub const DEFAULT_HIGHLIGHT_COLOR: &str = "#269eff";

/// Everything `MeshSmoothingSettingsPanel` draws from.
#[derive(Clone, Debug)]
pub struct SmoothingState {
    pub brush_size_mm: f32,
    pub strength: f32,
    pub iterations: f32,
    pub falloff: MeshSmoothingFalloff,
    /// `#rrggbb`, lowercase as the store keeps it.
    pub highlight_color: SharedString,
}

impl Default for SmoothingState {
    fn default() -> Self {
        Self {
            brush_size_mm: 0.5,
            strength: 0.5,
            iterations: 1.0,
            falloff: MeshSmoothingFalloff::Smooth,
            highlight_color: DEFAULT_HIGHLIGHT_COLOR.into(),
        }
    }
}

#[derive(Clone)]
pub struct SmoothingActions {
    pub on_brush_size_change: Rc<dyn Fn(f32)>,
    pub on_strength_change: Rc<dyn Fn(f32)>,
    pub on_iterations_change: Rc<dyn Fn(f32)>,
    pub on_falloff_change: Rc<dyn Fn(MeshSmoothingFalloff)>,
    pub on_highlight_color_change: Rc<dyn Fn(SharedString)>,
    pub on_restore_defaults: Rc<dyn Fn()>,
}

impl Default for SmoothingActions {
    fn default() -> Self {
        Self {
            on_brush_size_change: Rc::new(|_| {}),
            on_strength_change: Rc::new(|_| {}),
            on_iterations_change: Rc::new(|_| {}),
            on_falloff_change: Rc::new(|_| {}),
            on_highlight_color_change: Rc::new(|_| {}),
            on_restore_defaults: Rc::new(|| {}),
        }
    }
}

/// `clampedColorInput`: `#` + the trimmed colour, uppercased.
fn clamped_color_input(color: &str) -> String {
    let raw = color.trim();
    if raw.starts_with('#') {
        raw.to_uppercase()
    } else {
        format!("#{}", raw.to_uppercase())
    }
}

/// `#rrggbb` → `0xrrggbb`; anything unparsable falls back to the default.
fn parse_hex(color: &str) -> u32 {
    let raw = color.trim().trim_start_matches('#');
    u32::from_str_radix(raw, 16).unwrap_or(0x269eff)
}

/// HSV (hue in degrees, s/v in 0..1) of an `#rrggbb` colour.
fn hsv(color: u32) -> (f32, f32, f32) {
    let r = ((color >> 16) & 0xff) as f32 / 255.0;
    let g = ((color >> 8) & 0xff) as f32 / 255.0;
    let b = (color & 0xff) as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let hue = if hue < 0.0 { hue + 360.0 } else { hue };
    let saturation = if max == 0.0 { 0.0 } else { delta / max };
    (hue, saturation, max)
}

/// `hsl(h, 100%, 50%)` as `0xrrggbb`, the picker's right-hand gradient stop.
fn hue_color(hue: f32) -> u32 {
    let h = hue / 60.0;
    let x = 1.0 - ((h % 2.0) - 1.0).abs();
    let (r, g, b) = match h as i32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    ((r * 255.0).round() as u32) << 16
        | ((g * 255.0).round() as u32) << 8
        | (b * 255.0).round() as u32
}

/// One falloff button: `min-h-[36px] rounded-md border px-3 text-[12px]
/// font-semibold uppercase tracking-wide`.
///
/// This is a bare `<button>` in the source, not `.ui-button`, so the padding,
/// radius and 12px type all apply; only the active fill is the source's inline
/// `style`.
fn falloff_button(
    id: &'static str,
    falloff: MeshSmoothingFalloff,
    active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> gpui::Stateful<Div> {
    let spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Auto)
        .padding_x(12.0)
        .padding_y(0.0, 0.0)
        .radius(ROUNDED_MD)
        .font_size(12.0);
    let (border, background, text) = if active {
        (
            mix(ACCENT, WHITE, 0.90),
            mix(ACCENT, SURFACE_1, 0.16),
            mix(ACCENT, TEXT_STRONG, 0.75),
        )
    } else {
        (BORDER_SUBTLE, SURFACE_1, TEXT_MUTED)
    };
    button_with(
        id,
        falloff.label().to_uppercase(),
        &spec.fill(background, border, text),
    )
    .min_h(px(36.0))
    .on_click(on_click)
}

/// The Mesh Smoothing body: `px-2.5 pb-2.5 space-y-2`, scrolling.
pub fn smoothing_body(state: &SmoothingState, actions: &SmoothingActions) -> impl IntoElement {
    let mut body = div()
        .id("smoothing-body")
        .px(px(10.0))
        .pb(px(10.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .overflow_y_scroll();

    // ── Brush ───────────────────────────────────────────────────────────────
    let brush = actions.on_brush_size_change.clone();
    let strength = actions.on_strength_change.clone();
    let iterations = actions.on_iterations_change.clone();

    let mut falloffs = div().flex().gap(px(6.0));
    for (index, falloff) in [
        MeshSmoothingFalloff::Linear,
        MeshSmoothingFalloff::Smooth,
        MeshSmoothingFalloff::Sharp,
    ]
    .into_iter()
    .enumerate()
    {
        let change = actions.on_falloff_change.clone();
        falloffs = falloffs.child(div().flex_1().child(falloff_button(
            match index {
                0 => "smoothing-falloff-linear",
                1 => "smoothing-falloff-smooth",
                _ => "smoothing-falloff-sharp",
            },
            falloff,
            state.falloff == falloff,
            move |_, _, _| change(falloff),
        )));
    }

    body = body.child(
        // `brushCardStyle`: `color-mix(#4f8cff, border-subtle 78%)` on
        // `color-mix(#4f8cff, surface-1 94%)`.
        sub_card_with(Some((MOVE_TINT, 0.06)), 8.0)
            .gap(px(0.0))
            .child(section_header("Brush"))
            .child(
                div()
                    .pt(px(6.0))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(stepped_field(
                        "smoothing-brush-size",
                        state.brush_size_mm,
                        Some(Unit::new("mm", 10.0)),
                        FieldAlign::Center,
                        MESH_SMOOTHING_BRUSH_STEP_MM,
                        MESH_SMOOTHING_BRUSH_MIN_MM,
                        MESH_SMOOTHING_BRUSH_MAX_MM,
                        false,
                        Rc::new(move |next| {
                            brush(next.clamp(
                                MESH_SMOOTHING_BRUSH_MIN_MM,
                                MESH_SMOOTHING_BRUSH_MAX_MM,
                            ))
                        }),
                    ))
                    .child(stepped_field(
                        "smoothing-strength",
                        state.strength,
                        Some(Unit::new("Strength", 10.0)),
                        FieldAlign::Center,
                        0.01,
                        0.0,
                        1.0,
                        false,
                        Rc::new(move |next| strength(next.clamp(0.0, 1.0))),
                    ))
                    .child(stepped_field(
                        "smoothing-iterations",
                        state.iterations,
                        Some(Unit::new("Iterations", 10.0)),
                        FieldAlign::Center,
                        1.0,
                        1.0,
                        20.0,
                        false,
                        Rc::new(move |next| iterations(next.round().clamp(1.0, 20.0))),
                    ))
                    .child(falloffs),
            ),
    );

    // ── Highlight ───────────────────────────────────────────────────────────
    let color = parse_hex(state.highlight_color.as_ref());
    let (hue, saturation, value) = hsv(color);
    let change_color = actions.on_highlight_color_change.clone();

    let swatch = div()
        .w(px(32.0))
        .h(px(32.0))
        .flex_shrink_0()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(mix(BORDER_SUBTLE, WHITE, 0.92)))
        .bg(rgb(color));

    let hex_field = div()
        .flex_1()
        .min_w(px(0.0))
        .h(px(32.0))
        .px(px(10.0))
        .flex()
        .items_center()
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .text_size(px(FONT_MD))
        .text_color(rgb(TEXT_STRONG))
        .child(clamped_color_input(state.highlight_color.as_ref()));

    // `h-28 rounded-md border p-1 overflow-hidden` around the picker.
    let picker = div()
        .id("smoothing-picker")
        .h(px(112.0))
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgba(with_alpha(SURFACE_0, 0xf0)))
        .p(px(4.0))
        .overflow_hidden()
        .child(
            div()
                .relative()
                .w_full()
                .h_full()
                .rounded(px(4.0))
                // `linear-gradient(to right, #fff, hsl(hue, 100%, 50%))`.
                .bg(linear_gradient(
                    90.0,
                    linear_color_stop(rgb(WHITE), 0.0),
                    linear_color_stop(rgb(hue_color(hue)), 1.0),
                ))
                // `linear-gradient(to top, #000, transparent)`.
                .child(
                    div()
                        .absolute()
                        .left(px(0.0))
                        .top(px(0.0))
                        .right(px(0.0))
                        .bottom(px(0.0))
                        .bg(linear_gradient(
                            180.0,
                            linear_color_stop(rgba(with_alpha(BLACK, 0x00)), 0.0),
                            linear_color_stop(rgb(BLACK), 1.0),
                        )),
                )
                // The knob at (saturation, 1 - value).
                .child(
                    div()
                        .id("smoothing-picker-knob")
                        .absolute()
                        .left(gpui::relative(saturation))
                        .top(gpui::relative(1.0 - value))
                        .w(px(12.0))
                        .h(px(12.0))
                        .rounded(px(9999.0))
                        .border_2()
                        .border_color(rgb(WHITE))
                        .shadow(vec![gpui::BoxShadow::new(
                            px(0.0),
                            px(0.0),
                            rgba(with_alpha(BLACK, 0x66)).into(),
                        )
                        .blur_radius(px(2.0))])
                        // Clicking the knob walks the hue: the only interaction
                        // this port offers without a drag surface (see the module
                        // deviations).
                        .cursor_pointer()
                        .on_click(move |_, _, _| {
                            let next = hue_color((hue + 30.0) % 360.0);
                            change_color(format!("#{next:06x}").into());
                        }),
                ),
        );

    body = body.child(
        // `highlightCardStyle`: `color-mix(#8f6cff, border-subtle 80%)` on
        // `color-mix(#8f6cff, surface-1 95%)`.
        sub_card_with(Some((ROTATE_TINT, 0.05)), 8.0)
            .gap(px(0.0))
            .child(section_header("Highlight"))
            .child(
                div()
                    .pt(px(6.0))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(swatch)
                            .child(hex_field),
                    )
                    .child(picker),
            ),
    );

    // ── Restore defaults ────────────────────────────────────────────────────
    // `Button variant="secondary" size="auto"` with `w-full !h-8 px-3 text-xs
    // inline-flex items-center justify-center gap-1.5`; the icon is `h-3 w-3`
    // in `currentColor`.
    let restore = actions.on_restore_defaults.clone();
    let spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm)
        .full()
        .gap(6.0)
        .icon("rotate-ccw", 12.0, TEXT_STRONG);
    body.child(button_with("smoothing-restore", "Restore defaults", &spec).on_click(move |_, _, _| {
        restore()
    }))
}

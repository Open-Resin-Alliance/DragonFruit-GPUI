//! Overlay controls ported from the React app.
//!
//! * [`layer_slider`] ports `src/components/controls/LayerSlider.tsx` — the
//!   docked, embedded, compact minimal rail that both of the component's real
//!   call sites use (`CrossSliderPanel.tsx`, `PrintingPreviewPane.tsx`).
//! * [`view_cube`] ports the `ZUpGizmoViewcube` used by
//!   `src/components/scene/SceneCanvas/SceneCanvas.tsx`. GPUI cannot host a
//!   three.js scene, so the cube is projected here and painted with
//!   `PathBuilder`; the face labels are shaped text placed at the projected
//!   face centroids.
//!
//! Every size and color below traces to a class string in those files or to a
//! token in `theme.rs`. Where the source relies on CSS features GPUI lacks
//! (transitions, `translate`, `vh`, three-stop gradients) the nearest
//! equivalent is used and the derivation is noted inline.
//!
//! # PORT DEVIATIONS
//!
//! Diffed against `controls/LayerSlider.tsx` (the `isMinimalRail` branch that
//! both call sites render), `SceneCanvas/ZUpGizmoViewcube.tsx`,
//! `SceneCanvas/ZUpGizmoHelper.tsx` and the `ZUpGizmoViewcube` call in
//! `SceneCanvas.tsx`.
//!
//! ## LayerSlider
//!
//! * **Right-click thumb editor** (`Layer #` / `mm` toggle + text field,
//!   `editingThumb` / `editMode` / `editRawValue`): PORT GAP. The popover's
//!   open/mode/text state has no home — this component is a stateless element
//!   builder and `DragonFruit` carries no slider-editor fields. Nothing is
//!   drawn; the tooltip still advertises the gesture.
//! * **Shift precision arcs** (`isShiftHeld` → two pairs of blue arcs): need
//!   the modifier at render time, which a stateless builder cannot observe.
//!   The shift *behaviour* (10× fine scrub, 0.1× wheel step) is ported from
//!   the live event modifiers.
//! * **Shift latch during a drag**: the source turns shift on for the rest of
//!   the drag once pressed; GPUI reads the live modifier per move, so
//!   releasing shift mid-drag returns to 1×.
//! * **`scale-105` while dragging + `transition-[transform,background,
//!   box-shadow] duration-150`**: GPUI has no CSS transforms or transitions,
//!   so the thumb keeps its rest size and switches instantly.
//! * **Three-stop thumb gradient** (`mix(c,white 20%) → c → mix(c,white 20%)`):
//!   GPUI gradients take two stops; the leading highlight is kept.
//! * **`tabular-nums`**: no font-feature switch on a `div`; digits are
//!   proportional.
//! * **Tooltip**: the source's `Tooltip` follows the cursor and clamps to the
//!   viewport. GPUI renders a trigger-anchored box; it is right-aligned so it
//!   stays on screen against the rail's right edge.
//! * **Floating value badges**: the source gates them on
//!   `typeof displayHeightMm === 'number'` (`currentHeightMm` / `layerHeightMm`
//!   from the slicer). The GPUI app has no slicing height, and the minimal
//!   rail's badge content is the layer index, so the badge is shown whenever
//!   `show_value` is set (the same number the source draws).
//! * **`onScrubStart` / `onScrubEnd`, `dragBatchMode`, `liveValue` /
//!   `liveLowerValue`**: the GPUI slider writes straight into `DragonFruit`
//!   and the viewport reads the same value, so there is no deferred owner
//!   state to bridge; moves apply immediately (the source's `'immediate'`
//!   mode).
//! * **Non-minimal variants** (`docked`/`embedded` false: `ui-panel w-44`
//!   header, mm readout, `Max …`, static input field): not ported — both real
//!   call sites pass `embedded docked compactMinimalRail expandToContainer
//!   showValue`.
//! * **`disabled`**: the source has no disabled prop. The previous
//!   `max <= min` inference was invented and is removed.
//! * **`allowTrackClickJump`**: derived from `lower.is_none()`, which matches
//!   both call sites exactly (`PrintingPreviewPane` passes no lower and
//!   `allowTrackClickJump`; `CrossSliderPanel` passes a lower and no jump).
//! * **Keyboard**: the source's `onKeyDown` only prevents the browser default;
//!   GPUI stops propagation so the shell's arrow-key handler does not also
//!   act on a focused slider.
//!
//! ## View cube
//!
//! * **three.js → GPUI paths**: the gizmo camera is orthographic, so the
//!   projection is exact; there is no texture mapping. Labels are shaped text
//!   at the projected centroid, screen-aligned instead of rotated with the
//!   face (GPUI has no text rotation).
//! * **Label font**: the source asks for `600 24px Inter`; Inter is not
//!   bundled, so the app's Geist is used at the same size. 24px on the 128px
//!   face texture is 11.25px on the 60px face; long labels shrink exactly like
//!   `fitFontToWidth` (112px texture margin → 52.5px on screen, floor to whole
//!   texture pixels, minimum 8px).
//! * **Hover tint**: `color={hover ? hoverColor : 'white'}` multiplies the
//!   face texture; reproduced as a component-wise multiply of the face, text
//!   and stroke colours.
//! * **Tween**: `tweenCamera` animates at `turnRate = 4π`; GPUI snaps the
//!   camera instantly. The quarter-turn arrows and the home chip live in
//!   `ZUpGizmoHelper` (not this component) and are not ported.
//! * **Edge/corner boxes**: `0.38` offset with `0.25`/`0.5` dimensions against
//!   a `0.5` half-extent cube — they protrude under a pixel past the cube's
//!   support plane, exactly as in the source, so they are hittable only on that
//!   sliver. Their hover fill is clipped to the part in front of the cube
//!   (Sutherland–Hodgman against the support plane), matching three.js's depth
//!   test.
//! * **Label placement**: the source maps the label onto the face texture, so
//!   it is squashed along one axis as the face turns away. GPUI text cannot be
//!   affine-mapped, so the label stays screen-aligned at the projected centroid
//!   and its size is scaled by the face's foreshortening
//!   (`dot(normal, eye)`), which fades it out the same way.
//! * **`onPointerOut`**: hover is read from the live pointer on every paint,
//!   so no hover state is kept.
//! * **Face opacity**: three.js applies `opacity 0.75` to fill, stroke and
//!   label together; GPUI paints the fill and stroke at 0.75 and the label at
//!   0.75 over them.
//! * **Placement**: `ZUpGizmoHelper` positions the HUD at the bottom-right
//!   (`margin = [48 + 12 + 42 + 30, 82]` to the cube centre, `[82, 82]` in
//!   printing mode). The caller places this 120×120 widget so its centre lands
//!   there; the widget itself is unpositioned.

use std::cell::{Cell, RefCell};
use std::f32::consts::FRAC_PI_2;
use std::rc::Rc;

use gpui::{
    App, Bounds, BoxShadow, ClickEvent, Corners, DragMoveEvent, EmptyView, EntityId, FontWeight,
    IntoElement, MouseButton, Pixels, Point, ScrollDelta, SharedString, WeakEntity, canvas, div,
    gpu_canvas, linear_color_stop, linear_gradient, prelude::*, px, relative, rgb, rgba,
};

use gpui_core::camera::Camera;
use gpui_core::icons;
use gpui_core::theme::*;
use gpui_core::viewport::Viewport;
use gpui_interop::attach;
use gpui_wgpu::{ImportedTextureExt as _, WgpuRenderer};
use crate::DragonFruit;

// ---------------------------------------------------------------------------
// LayerSlider
// ---------------------------------------------------------------------------

/// `getThumbHitThresholds`: the visual thumb is 9px tall, so the handle core is
/// ±4.5px and the grab band ±6px (4.5 + the 1.5px comfort padding).
const THUMB_CORE_HALF: f32 = 4.5;
const THUMB_GRAB_HALF: f32 = 6.0;
/// `h-[9px] w-[24px]`.
const THUMB_HEIGHT: f32 = 9.0;
const THUMB_WIDTH: f32 = 24.0;
/// `w-1.5` on the track and the progress fill.
const TRACK_WIDTH: f32 = 6.0;
/// `w-[34px]` on the compact minimal rail's outer column.
const RAIL_COLUMN_WIDTH: f32 = 34.0;
/// `w-4` on the compact minimal rail.
const RAIL_WIDTH: f32 = 16.0;
/// `min-h-[300px]` on the rail when it expands to its container.
const RAIL_MIN_HEIGHT: f32 = 300.0;
/// `px-0.5` on the minimal rail badge.
const BADGE_PX: f32 = 2.0;
/// `text-[8px]` on the minimal rail badge.
const BADGE_TEXT: f32 = 8.0;

/// `emitChange`'s `Math.max(1e-6, max - min)`.
fn value_span(min: f32, max: f32) -> f32 {
    (max - min).max(1e-6)
}

/// `Math.min(max, Math.max(min, v))`, without `f32::clamp`'s panic when the
/// caller hands us an inverted range.
fn clamp_to(value: f32, min: f32, max: f32) -> f32 {
    if max < min {
        min
    } else {
        value.max(min).min(max)
    }
}

/// `snap` then `clamp`.
fn snap_clamp(value: f32, min: f32, max: f32, step: f32) -> f32 {
    let step = if step.is_finite() && step > 0.0 { step } else { 1.0 };
    clamp_to((value / step).round() * step, min, max)
}

/// `emitChange`: the upper thumb may never go below the lower thumb.
fn upper_target(raw: f32, floor: f32, min: f32, max: f32, step: f32) -> f32 {
    snap_clamp(raw, min, max, step).max(floor)
}

/// `emitLowerChange`: the lower thumb may never go above the upper thumb.
fn lower_target(raw: f32, ceiling: f32, min: f32, max: f32, step: f32) -> f32 {
    snap_clamp(raw, min, max, step).min(ceiling)
}

/// `emitWindowChange`: keep the dragged window ordered, inside the range, and
/// the same width.
fn window_target(raw_lower: f32, raw_upper: f32, min: f32, max: f32, step: f32) -> (f32, f32) {
    let min_max_span = (max - min).max(0.0);
    let (mut target_lower, mut target_upper) = if raw_lower > raw_upper {
        (raw_upper, raw_lower)
    } else {
        (raw_lower, raw_upper)
    };
    let window_span = min_max_span.min((target_upper - target_lower).max(0.0));

    if target_lower < min {
        target_lower = min;
        target_upper = min + window_span;
    }
    if target_upper > max {
        // The source also assigns `targetUpper = max` here, but never reads it
        // again — only the lower bound moves the window.
        target_lower = max - window_span;
    }

    let snapped_lower = snap_clamp(target_lower, min, max, step);
    let mut snapped_upper = snapped_lower + window_span;
    if snapped_upper > max {
        snapped_upper = max;
    }
    if snapped_upper < snapped_lower {
        snapped_upper = snapped_lower;
    }

    let final_upper = snap_clamp(snapped_upper, min, max, step);
    let final_lower = snap_clamp(snapped_lower, min, max, step).min(final_upper);
    (final_lower, final_upper)
}

/// `setByClientY`: the rail maps bottom → `min` and top → `max`. `shift` is the
/// source's 10× fine mode, which moves a tenth of the way to the pointer.
fn pointer_value(
    bounds: Bounds<Pixels>,
    position: Pixels,
    current: f32,
    min: f32,
    max: f32,
    shift: bool,
) -> f32 {
    let height = bounds.size.height.0.max(1.0);
    let rel = (position.0 - bounds.origin.y.0) / height;
    let inv = 1.0 - rel;
    let span = value_span(min, max);
    if shift {
        let current_percent = (current - min) / span;
        min + (current_percent + (inv - current_percent) * 0.1) * span
    } else {
        min + inv * span
    }
}

/// Which thumb a press lands on (`onPointerDown`'s proximity test).
#[derive(Clone, Copy, PartialEq, Eq)]
enum DragThumb {
    Upper,
    Lower,
    Window,
}

fn pick_thumb(
    bounds: Bounds<Pixels>,
    y: f32,
    upper: f32,
    lower: f32,
    has_lower: bool,
    min: f32,
    max: f32,
) -> Option<DragThumb> {
    let height = bounds.size.height.0.max(1.0);
    let click_inv = 1.0 - (y - bounds.origin.y.0) / height;
    let span = value_span(min, max);
    let upper_pos = (upper - min) / span;
    let upper_distance = (click_inv - upper_pos).abs() * height;

    if has_lower {
        let lower_pos = (lower - min) / span;
        let lower_distance = (click_inv - lower_pos).abs() * height;
        let in_window =
            click_inv >= lower_pos.min(upper_pos) && click_inv <= lower_pos.max(upper_pos);
        let in_window_core =
            in_window && lower_distance > THUMB_CORE_HALF && upper_distance > THUMB_CORE_HALF;

        if lower_distance > THUMB_GRAB_HALF && upper_distance > THUMB_GRAB_HALF {
            // Not near either thumb: only the window between them drags.
            if in_window {
                Some(DragThumb::Window)
            } else {
                None
            }
        } else if in_window_core {
            Some(DragThumb::Window)
        } else if lower_distance < upper_distance {
            Some(DragThumb::Lower)
        } else {
            Some(DragThumb::Upper)
        }
    } else if upper_distance > THUMB_GRAB_HALF {
        None
    } else {
        Some(DragThumb::Upper)
    }
}

/// What the drag latched when it started: the source's `activeDraggingThumbRef`
/// plus the window-drag anchors.
#[derive(Clone, Copy)]
struct DragStart {
    thumb: DragThumb,
    y: f32,
    upper: f32,
    lower: f32,
    height: f32,
}

/// Payload for the rail drag. `start` is filled by the drag constructor (the
/// payload is built during render, before the gesture exists).
#[derive(Clone)]
struct SliderDrag {
    entity: EntityId,
    start: Rc<Cell<Option<DragStart>>>,
}

fn set_upper(entity: &WeakEntity<DragonFruit>, next: f32, cx: &mut App) {
    entity
        .update(cx, |this, cx| {
            if this.layer != next {
                this.layer = next;
                cx.notify();
            }
        })
        .ok();
}

fn set_lower(entity: &WeakEntity<DragonFruit>, next: f32, cx: &mut App) {
    entity
        .update(cx, |this, cx| {
            if this.lower_layer != next {
                this.lower_layer = next;
                cx.notify();
            }
        })
        .ok();
}

fn set_window(entity: &WeakEntity<DragonFruit>, lower: f32, upper: f32, cx: &mut App) {
    entity
        .update(cx, |this, cx| {
            let mut changed = false;
            if this.lower_layer != lower {
                this.lower_layer = lower;
                changed = true;
            }
            if this.layer != upper {
                this.layer = upper;
                changed = true;
            }
            if changed {
                cx.notify();
            }
        })
        .ok();
}

fn toggle_cross_section(entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    entity
        .update(cx, |this, cx| {
            this.cross_section = !this.cross_section;
            cx.notify();
        })
        .ok();
}

/// `nudge`: one step relative to what the entity currently holds, so repeated
/// wheel or key events in one frame accumulate (the source's `valueRef`).
fn nudge(
    entity: &WeakEntity<DragonFruit>,
    delta: f32,
    floor: f32,
    min: f32,
    max: f32,
    step: f32,
    cx: &mut App,
) {
    entity
        .update(cx, |this, cx| {
            let next = upper_target(this.layer + delta, floor, min, max, step);
            if this.layer != next {
                this.layer = next;
                cx.notify();
            }
        })
        .ok();
}

/// `{max}` / `{min}` badges: JS renders the raw number.
fn number_label(value: f32) -> String {
    if value.fract().abs() < 0.0001 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value}")
    }
}

/// The compact minimal rail: `rounded-md border px-0.5 py-0 text-[8px]
/// font-semibold tabular-nums` with `railBadgeStyle` / `railCurrentBadgeStyle`.
fn rail_badge(label: impl Into<SharedString>, current: bool) -> impl IntoElement {
    let color = if current { TEXT_STRONG } else { TEXT_MUTED };
    // `color-mix(in srgb, var(--border-subtle), transparent 15%)` / `… 10%`.
    let border = if current {
        rgb(BORDER_SUBTLE).alpha(0.9)
    } else {
        rgb(BORDER_SUBTLE).alpha(0.85)
    };
    // `color-mix(in srgb, var(--surface-1), transparent 10%)` / `… 4%`.
    let background = if current {
        rgb(SURFACE_1).alpha(0.96)
    } else {
        rgb(SURFACE_1).alpha(0.9)
    };
    div()
        .flex()
        .items_center()
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(border)
        .bg(background)
        .px(px(BADGE_PX))
        .text_size(px(BADGE_TEXT))
        .font_weight(FontWeight::SEMIBOLD)
        .whitespace_nowrap()
        .text_color(rgb(color))
        .child(label.into())
}

/// The 24×9 pill: `rounded-full border` with the thumb gradient, ring and drop
/// shadow, plus the optional value badge. `opacity` is the pill's own
/// `opacity: 0.75` on the lower thumb; the badge is a sibling and stays opaque.
fn thumb_body(color: u32, badge: Option<(bool, SharedString)>, opacity: f32) -> impl IntoElement {
    let pill = div()
        .w(px(THUMB_WIDTH))
        .h(px(THUMB_HEIGHT))
        .rounded_full()
        .border_1()
        .opacity(opacity)
        // `color-mix(in srgb, white, thumbColor 20%)`.
        .border_color(rgb(mix(0xffffff, color, 0.8)))
        // `linear-gradient(90deg, mix(thumbColor, white 20%), thumbColor, …)`:
        // GPUI gradients take two stops, so the leading highlight is kept.
        .bg(linear_gradient(
            90.0,
            linear_color_stop(rgb(mix(color, 0xffffff, 0.8)), 0.0),
            linear_color_stop(rgb(color), 1.0),
        ))
        // `box-shadow: 0 0 0 2px mix(thumbColor, transparent 80%),
        //  0 4px 10px rgba(0,0,0,0.35)` at rest.
        .shadow(vec![
            BoxShadow::new(px(0.0), px(0.0), rgb(color).alpha(0.2).into())
                .spread_radius(px(2.0)),
            BoxShadow::new(px(0.0), px(4.0), rgba(0x00000059).into()).blur_radius(px(10.0)),
        ]);

    let mut thumb = div()
        .relative()
        .w(px(THUMB_WIDTH))
        .h(px(THUMB_HEIGHT))
        .child(pill);

    if let Some((below, label)) = badge {
        // `-top-5` above the thumb, or `top-3` when the thumb is at the top.
        thumb = thumb.child(
            div()
                .absolute()
                .left(px(0.0))
                .right(px(0.0))
                .top(px(if below { 12.0 } else { -20.0 }))
                .flex()
                .justify_center()
                .child(rail_badge(label, true)),
        );
    }

    thumb
}

/// `LayerSlider.tsx`, ported as the docked / embedded / compact minimal rail.
///
/// `value` is the upper thumb, `lower` the optional lower thumb (both in the
/// same units as `min`/`max`). `cross_section` is the source's
/// `crossSectionEnabled`: it colours the thumb and fill and is what the
/// double-click toggles.
///
/// Scrub, track click, wheel and arrow keys write the upper value into
/// `DragonFruit::layer`; the lower thumb and the window between the thumbs
/// write `DragonFruit::lower_layer`.
///
/// The rail fills its parent's height (`h-full`, `flex-1`), with
/// `min-h-[300px]` so it stays usable in an auto-height column.
pub fn layer_slider(
    value: f32,
    lower: Option<f32>,
    cross_section: bool,
    min: f32,
    max: f32,
    step: f32,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let step = if step.is_finite() && step > 0.0 { step } else { 1.0 };
    // `percent` uses `Math.max(1, max - min)`; the value math uses 1e-6.
    let display_span = (max - min).max(1.0);
    let upper = clamp_to(value, min, max);
    let upper_percent = clamp_to((upper - min) / display_span, 0.0, 1.0);
    let lower_value = lower.map(|l| clamp_to(l, min, max));
    let lower_percent = lower_value.map(|l| clamp_to((l - min) / display_span, 0.0, 1.0));
    // `thumbColor = crossSectionEnabled ? var(--accent) : var(--text-muted)`.
    let thumb_color = if cross_section { ACCENT } else { TEXT_MUTED };
    let entity_id = entity.entity_id();
    let bounds: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));
    let drag_start: Rc<Cell<Option<DragStart>>> = Rc::new(Cell::new(None));
    let has_lower = lower.is_some();

    // -- track ---------------------------------------------------------------
    let track = div()
        .absolute()
        .left(relative(0.5))
        .ml(px(-TRACK_WIDTH * 0.5))
        .top(px(0.0))
        .bottom(px(0.0))
        .w(px(TRACK_WIDTH))
        .rounded_full()
        // `color-mix(in srgb, var(--surface-2), black 8%)`.
        .bg(rgb(mix(SURFACE_2, 0x000000, 0.92)))
        .border_1()
        // `color-mix(in srgb, var(--border-subtle), transparent 40%)`.
        .border_color(rgb(BORDER_SUBTLE).alpha(0.6));

    // -- progress fill -------------------------------------------------------
    let fill_from = lower_percent.unwrap_or(0.0);
    let fill_height = (upper_percent - fill_from).max(0.0);
    // `crossSectionEnabled ? linear-gradient(accent +14% white, accent)
    //  : linear-gradient(text-muted +10% white, text-muted)`.
    let fill_top = if cross_section {
        mix(ACCENT, 0xffffff, 0.86)
    } else {
        mix(TEXT_MUTED, 0xffffff, 0.9)
    };
    let mut fill = div()
        .absolute()
        .left(relative(0.5))
        .ml(px(-TRACK_WIDTH * 0.5))
        .w(px(TRACK_WIDTH))
        .bottom(relative(fill_from))
        .h(relative(fill_height))
        .rounded_full()
        .bg(linear_gradient(
            180.0,
            linear_color_stop(rgb(fill_top), 0.0),
            linear_color_stop(rgb(thumb_color), 1.0),
        ))
        // `opacity: crossSectionEnabled ? 1 : 0.45`.
        .opacity(if cross_section { 1.0 } else { 0.45 });
    if cross_section {
        // `box-shadow: 0 0 10px color-mix(in srgb, var(--accent), transparent 65%)`.
        fill = fill.shadow(vec![
            BoxShadow::new(px(0.0), px(0.0), rgb(ACCENT).alpha(0.35).into())
                .blur_radius(px(10.0)),
        ]);
    }

    // -- lower thumb ---------------------------------------------------------
    let lower_thumb = lower_percent.map(|percent| {
        let badge = lower_value.and_then(|value| {
            let hide = percent <= 1e-6 || percent >= 1.0 - 1e-6;
            (!hide).then(|| {
                (
                    true,
                    SharedString::from(format!("{}", value.round() as i64)),
                )
            })
        });
        div()
            .absolute()
            .left(relative(0.5))
            .ml(px(-THUMB_WIDTH * 0.5))
            .top(relative(1.0 - percent))
            .mt(px(-THUMB_HEIGHT * 0.5))
            .w(px(THUMB_WIDTH))
            .h(px(THUMB_HEIGHT))
            // The lower thumb's pill is drawn at `opacity: 0.75` in the source.
            .child(thumb_body(thumb_color, badge, 0.75))
    });

    // -- upper thumb ---------------------------------------------------------
    // `shouldPlaceCurrentBadgeBelowThumb = percent >= 96`.
    let badge_below = upper_percent >= 0.96;
    let hide_upper_badge = upper_percent <= 1e-6 || upper_percent >= 1.0 - 1e-6;
    let upper_badge = (!hide_upper_badge).then(|| {
        (
            badge_below,
            SharedString::from(format!("{}", upper.round() as i64)),
        )
    });
    let upper_thumb = div()
        .absolute()
        .left(relative(0.5))
        .ml(px(-THUMB_WIDTH * 0.5))
        .top(relative(1.0 - upper_percent))
        .mt(px(-THUMB_HEIGHT * 0.5))
        .w(px(THUMB_WIDTH))
        .h(px(THUMB_HEIGHT))
        .child(thumb_body(thumb_color, upper_badge, 1.0));

    // -- rail ----------------------------------------------------------------
    let bounds_for_canvas = bounds.clone();
    let rail = div()
        .id("layer-slider-rail")
        .relative()
        .size_full()
        .cursor_pointer()
        .focusable()
        .tab_stop(true)
        .child(
            canvas(
                move |canvas_bounds, _window, _cx| {
                    bounds_for_canvas.set(Some(canvas_bounds));
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .size_full(),
        )
        .child(track)
        .child(fill)
        .children(lower_thumb)
        .child(upper_thumb)
        // `onDoubleClick` → `onToggleCrossSection`.
        .on_click({
            let entity = entity.clone();
            move |event, _, cx| {
                let count = match event {
                    ClickEvent::Mouse(event) => event.down.click_count,
                    ClickEvent::Touch(event) => event.tap_count,
                    ClickEvent::Keyboard(_) => 0,
                };
                if count < 2 {
                    return;
                }
                cx.stop_propagation();
                toggle_cross_section(&entity, cx);
            }
        })
        // Track click: only the no-lower call site enables `allowTrackClickJump`,
        // and only a press away from the thumb jumps.
        .on_mouse_down(MouseButton::Left, {
            let bounds = bounds.clone();
            let entity = entity.clone();
            move |event, _, cx| {
                if has_lower {
                    return;
                }
                let Some(bounds) = bounds.get() else {
                    return;
                };
                let y = event.position.y.0;
                let (current_upper, current_lower) = entity
                    .read_with(cx, |this, _| (this.layer, this.lower_layer))
                    .unwrap_or((upper, lower_value.unwrap_or(min)));
                if pick_thumb(bounds, y, current_upper, current_lower, false, min, max).is_some() {
                    return;
                }
                let raw = pointer_value(bounds, event.position.y, current_upper, min, max, false);
                set_upper(&entity, upper_target(raw, min, min, max, step), cx);
            }
        })
        .on_drag(
            SliderDrag {
                entity: entity_id,
                start: drag_start.clone(),
            },
            {
                let entity = entity.clone();
                let bounds = bounds.clone();
                move |drag: &SliderDrag, offset: Point<Pixels>, _window, cx| {
                    if let Some(bounds) = bounds.get() {
                        let y = bounds.origin.y.0 + offset.y.0;
                        let (current_upper, current_lower) = entity
                            .read_with(cx, |this, _| (this.layer, this.lower_layer))
                            .unwrap_or((upper, lower_value.unwrap_or(min)));
                        let thumb = pick_thumb(
                            bounds,
                            y,
                            current_upper,
                            current_lower,
                            has_lower,
                            min,
                            max,
                        );
                        drag.start.set(thumb.map(|thumb| DragStart {
                            thumb,
                            y,
                            upper: current_upper,
                            lower: current_lower,
                            height: bounds.size.height.0,
                        }));
                    }
                    cx.stop_propagation();
                    cx.new(|_| EmptyView)
                }
            },
        )
        // `on_drag_move` fires for every move while a drag of this payload type
        // is active, so the thumb keeps tracking outside the rail.
        .on_drag_move::<SliderDrag>({
            let entity = entity.clone();
            move |event: &DragMoveEvent<SliderDrag>, _, cx| {
                if event.drag(cx).entity != entity_id {
                    return;
                }
                let Some(start) = event.drag(cx).start.get() else {
                    return;
                };
                let y = event.event.position.y.0;
                let shift = event.event.modifiers.shift;
                let (current_upper, current_lower) = entity
                    .read_with(cx, |this, _| (this.layer, this.lower_layer))
                    .unwrap_or((start.upper, start.lower));
                let height = start.height.max(1.0);
                let span = value_span(min, max);
                // `emitChange`'s floor is the lower thumb, or `min` when the
                // slider has no lower bound.
                let floor = if has_lower { current_lower } else { min };

                match start.thumb {
                    DragThumb::Upper => {
                        let raw = pointer_value(
                            event.bounds,
                            event.event.position.y,
                            current_upper,
                            min,
                            max,
                            shift,
                        );
                        set_upper(&entity, upper_target(raw, floor, min, max, step), cx);
                    }
                    DragThumb::Lower => {
                        let raw = pointer_value(
                            event.bounds,
                            event.event.position.y,
                            current_lower,
                            min,
                            max,
                            shift,
                        );
                        set_lower(
                            &entity,
                            lower_target(raw, current_upper, min, max, step),
                            cx,
                        );
                    }
                    DragThumb::Window => {
                        let drag_delta_inv = -((y - start.y) / height);
                        let precision = if shift { 0.1 } else { 1.0 };
                        let delta = drag_delta_inv * span * precision;
                        let window_span = start.upper - start.lower;
                        let mut next_lower = start.lower + delta;
                        let mut next_upper = next_lower + window_span;
                        if next_lower < min {
                            next_lower = min;
                            next_upper = min + window_span;
                        }
                        if next_upper > max {
                            next_upper = max;
                            next_lower = max - window_span;
                        }
                        let (lower, upper) = window_target(next_lower, next_upper, min, max, step);
                        set_window(&entity, lower, upper, cx);
                    }
                }
            }
        })
        // Wheel: `deltaY > 0` steps down; shift is the 0.1× fine step.
        .on_scroll_wheel({
            let entity = entity.clone();
            move |event, _, cx| {
                cx.stop_propagation();
                let delta = match event.delta {
                    ScrollDelta::Lines(delta) => delta.y,
                    ScrollDelta::Pixels(delta) => delta.y.0,
                };
                if delta == 0.0 {
                    return;
                }
                let direction = if delta > 0.0 { -1.0 } else { 1.0 };
                let amount = if event.modifiers.shift { step * 0.1 } else { step };
                nudge(&entity, direction * amount, lower_value.unwrap_or(min), min, max, step, cx);
            }
        })
        // Arrow keys, once a click has focused the rail.
        .on_key_down({
            let entity = entity.clone();
            move |event, _, cx| {
                let direction = match event.keystroke.key.as_str() {
                    "up" | "right" => 1.0,
                    "down" | "left" => -1.0,
                    _ => return,
                };
                cx.stop_propagation();
                nudge(&entity, direction * step, lower_value.unwrap_or(min), min, max, step, cx);
            }
        });

    // -- tooltip -------------------------------------------------------------
    // `formatMinimalRailTitle`: the height is unknown without a slicer, so the
    // source's `'—'` fallback is used.
    let cross_section_hint = if cross_section {
        "Double-click to turn the cross-section off"
    } else {
        "Double-click to turn the cross-section back on"
    };
    let title = format!(
        "Layer {} • — • Right-click a handle to type an exact value • {}",
        upper.round() as i64,
        cross_section_hint
    );
    let tooltip = div()
        .absolute()
        .right(px(0.0))
        .top(relative(1.0))
        .mt(px(28.0))
        .opacity(0.0)
        .group_hover("layer-slider-tooltip", |style| style.opacity(1.0))
        .child(
            div()
                .rounded(px(4.0))
                .px(px(8.0))
                .py(px(6.0))
                .text_size(px(11.0))
                .line_height(px(11.0 * LINE_HEIGHT_TIGHT))
                .font_weight(FontWeight::MEDIUM)
                .bg(rgba(0x181818fa))
                .text_color(rgb(TEXT_STRONG))
                .border_1()
                .border_color(rgb(ACCENT))
                .max_w(px(260.0))
                .whitespace_normal()
                .text_left()
                .shadow(vec![
                    BoxShadow::new(px(0.0), px(6.0), rgba(0x00000070).into())
                        .blur_radius(px(32.0)),
                    BoxShadow::new(px(0.0), px(1.5), rgba(0x00000047).into())
                        .blur_radius(px(8.0)),
                ])
                .child(SharedString::from(title)),
        );

    let rail_wrapper = div()
        .id("layer-slider-tooltip")
        .relative()
        .flex_1()
        .h_full()
        .min_h(px(RAIL_MIN_HEIGHT))
        .w(px(RAIL_WIDTH))
        .mx_auto()
        .cursor_pointer()
        .group("layer-slider-tooltip")
        .child(rail)
        .child(tooltip);

    // -- column --------------------------------------------------------------
    // `mx-auto w-[34px] pt-1 pb-0` + `h-full min-h-0 flex flex-col`.
    div()
        .relative()
        .h_full()
        .w_full()
        .child(
            div()
                .flex()
                .flex_col()
                .w(px(RAIL_COLUMN_WIDTH))
                .h_full()
                .min_h(px(0.0))
                .pt(px(4.0))
                .mx_auto()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .mb(px(4.0))
                        .child(rail_badge(number_label(max), false)),
                )
                .child(rail_wrapper)
                .child(
                    div()
                        .mt(px(4.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(6.0))
                        .child(rail_badge(number_label(min), false)),
                ),
        )
}

// ---------------------------------------------------------------------------
// View cube
// ---------------------------------------------------------------------------

/// `scale={[60, 60, 60]}` on a unit box: a 60px cube, so ±30.
const CUBE_HALF: f32 = 30.0;
/// The gizmo's HUD footprint; the cube sits inside it with room for the labels.
const CUBE_WIDGET: f32 = 120.0;
/// `opacity={0.6}` on the edge/corner material.
const EDGE_OPACITY: f32 = 0.6;
/// `makePositionVector`: `0.38` in cube units.
const EDGE_OFFSET: f32 = 0.38 * 60.0;
/// `edgeDimensions` / `cornerDimensions`, in px before the mesh's `1.01`.
const EDGE_LONG: f32 = 0.5 * 60.0;
const EDGE_SHORT: f32 = 0.25 * 60.0;
/// `scale={1.01}` on every edge/corner mesh.
const EDGE_MESH_SCALE: f32 = 1.01;
/// `polarTilt` in `ZUpGizmoHelper`.
const POLAR_TILT: f32 = 0.0087;

/// One face of the Z-up cube. `ZUpGizmoViewcube` rotates the three.js box by
/// −90° about Z, which maps its +X face to −Y (the printer's front), +Y to +X,
/// and leaves ±Z alone — so the labels land on the world axes below.
struct CubeFace {
    label: &'static str,
    normal: [f32; 3],
}

const CUBE_FACES: [CubeFace; 6] = [
    CubeFace {
        label: "FRONT",
        normal: [0.0, -1.0, 0.0],
    },
    CubeFace {
        label: "BACK",
        normal: [0.0, 1.0, 0.0],
    },
    CubeFace {
        label: "RIGHT",
        normal: [1.0, 0.0, 0.0],
    },
    CubeFace {
        label: "LEFT",
        normal: [-1.0, 0.0, 0.0],
    },
    CubeFace {
        label: "TOP",
        normal: [0.0, 0.0, 1.0],
    },
    CubeFace {
        label: "BOTTOM",
        normal: [0.0, 0.0, -1.0],
    },
];

/// `edges` in `ZUpGizmoViewcube.tsx`, in the cube's local frame.
const EDGE_DIRS: [[f32; 3]; 12] = [
    [1.0, 1.0, 0.0],
    [1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [1.0, -1.0, 0.0],
    [0.0, 1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, 1.0],
    [0.0, -1.0, -1.0],
    [-1.0, 1.0, 0.0],
    [-1.0, 0.0, 1.0],
    [-1.0, 0.0, -1.0],
    [-1.0, -1.0, 0.0],
];

/// `corners` in `ZUpGizmoViewcube.tsx`, in the cube's local frame.
const CORNER_DIRS: [[f32; 3]; 8] = [
    [1.0, 1.0, 1.0],
    [1.0, 1.0, -1.0],
    [1.0, -1.0, 1.0],
    [1.0, -1.0, -1.0],
    [-1.0, 1.0, 1.0],
    [-1.0, 1.0, -1.0],
    [-1.0, -1.0, 1.0],
    [-1.0, -1.0, -1.0],
];

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let length = dot(v, v).sqrt();
    if length <= f32::EPSILON {
        [0.0, 0.0, 0.0]
    } else {
        [v[0] / length, v[1] / length, v[2] / length]
    }
}

/// `GIZMO_Z_ROTATION = -π/2` about Z: `(x, y, z) → (y, -x, z)`.
fn rotate_gizmo(v: [f32; 3]) -> [f32; 3] {
    [v[1], -v[0], v[2]]
}

/// `color` × `color`, the multiplication three.js applies between a material
/// colour and its texture.
fn multiply(a: u32, b: u32) -> u32 {
    let channel = |shift: u32| {
        ((((a >> shift) & 0xff) * ((b >> shift) & 0xff)) / 0xff) as u32
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

/// The camera's view basis in the same Z-up frame the scene uses
/// (`Camera::eye` is `(cos(pitch)·cos(yaw), cos(pitch)·sin(yaw), sin(pitch))`).
struct CubeView {
    eye: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    center: [f32; 2],
}

fn cube_view(camera: &Camera, center: [f32; 2]) -> CubeView {
    // `stabilizeDirection` keeps the camera a hair off the poles; clamping the
    // pitch the same way keeps the basis defined and matches the tween's view.
    let pole = FRAC_PI_2 - POLAR_TILT;
    let pitch = camera.pitch.clamp(-pole, pole);
    let yaw = camera.yaw;
    let eye = [
        pitch.cos() * yaw.cos(),
        pitch.cos() * yaw.sin(),
        pitch.sin(),
    ];
    let forward = [-eye[0], -eye[1], -eye[2]];
    let right = normalize(cross(forward, [0.0, 0.0, 1.0]));
    let up = normalize(cross(right, forward));
    CubeView {
        eye,
        right,
        up,
        center,
    }
}

fn project(view: &CubeView, p: [f32; 3]) -> [f32; 2] {
    [
        view.center[0] + dot(p, view.right),
        view.center[1] - dot(p, view.up),
    ]
}

/// `tweenCamera`'s target: the world direction the camera should sit on.
fn direction_to_yaw_pitch(direction: [f32; 3]) -> (f32, f32) {
    let mut stabilized = normalize(direction);
    let up_dot = stabilized[2];
    if up_dot.abs() >= 0.999 {
        // `stabilizeDirection`: tilt a fraction of a degree toward +Y so the
        // azimuth is defined.
        let sign = if up_dot > 0.0 { -1.0 } else { 1.0 };
        stabilized = normalize([
            stabilized[0],
            stabilized[1] + POLAR_TILT.tan() * sign,
            stabilized[2],
        ]);
    }
    (stabilized[1].atan2(stabilized[0]), stabilized[2].asin())
}

/// The four corners of the face with the given axis normal, wound consistently.
fn face_corners(normal: [f32; 3]) -> [[f32; 3]; 4] {
    let axis = if normal[0] != 0.0 {
        0
    } else if normal[1] != 0.0 {
        1
    } else {
        2
    };
    let (a, b) = match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    let sign = normal[axis];
    let signs = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    let mut corners = [[0.0f32; 3]; 4];
    for (index, (sa, sb)) in signs.into_iter().enumerate() {
        let mut corner = [0.0f32; 3];
        corner[axis] = sign * CUBE_HALF;
        corner[a] = sa * CUBE_HALF;
        corner[b] = sb * CUBE_HALF;
        corners[index] = corner;
    }
    corners
}

/// A gizmo edge/corner box in world space (the −90° Z rotation maps its axes
/// onto the world axes, so the box stays axis-aligned).
#[derive(Clone, Copy)]
struct GizmoBox {
    center: [f32; 3],
    half: [f32; 3],
}

fn edge_box(direction: [f32; 3]) -> GizmoBox {
    let center = rotate_gizmo(direction);
    let center = [
        center[0] * EDGE_OFFSET,
        center[1] * EDGE_OFFSET,
        center[2] * EDGE_OFFSET,
    ];
    let half = |axis: usize| {
        let half = if direction[axis] == 0.0 {
            EDGE_LONG * 0.5
        } else {
            EDGE_SHORT * 0.5
        };
        half * EDGE_MESH_SCALE
    };
    GizmoBox {
        center,
        half: [half(0), half(1), half(2)],
    }
}

fn corner_box(direction: [f32; 3]) -> GizmoBox {
    let center = rotate_gizmo(direction);
    let half = EDGE_SHORT * 0.5 * EDGE_MESH_SCALE;
    GizmoBox {
        center: [
            center[0] * EDGE_OFFSET,
            center[1] * EDGE_OFFSET,
            center[2] * EDGE_OFFSET,
        ],
        half: [half, half, half],
    }
}

/// The corners of one axis-aligned face of a box.
fn box_face_corners(center: [f32; 3], half: [f32; 3], normal: [f32; 3]) -> [[f32; 3]; 4] {
    let axis = if normal[0] != 0.0 {
        0
    } else if normal[1] != 0.0 {
        1
    } else {
        2
    };
    let (a, b) = match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    let signs = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    let mut corners = [[0.0f32; 3]; 4];
    for (index, (sa, sb)) in signs.into_iter().enumerate() {
        let mut corner = [0.0f32; 3];
        corner[axis] = center[axis] + normal[axis] * half[axis];
        corner[a] = center[a] + sa * half[a];
        corner[b] = center[b] + sb * half[b];
        corners[index] = corner;
    }
    corners
}

/// Depth of the box surface nearest the camera along an orthographic ray
/// (`p0 + t·eye`, with the camera at large positive `t`).
fn box_front_depth(box_: &GizmoBox, p0: [f32; 3], eye: [f32; 3]) -> Option<f32> {
    let mut t_min = f32::NEG_INFINITY;
    let mut t_max = f32::INFINITY;
    for axis in 0..3 {
        let d = eye[axis];
        let lo = box_.center[axis] - box_.half[axis];
        let hi = box_.center[axis] + box_.half[axis];
        if d.abs() < 1e-6 {
            if p0[axis] < lo || p0[axis] > hi {
                return None;
            }
        } else {
            let t1 = (lo - p0[axis]) / d;
            let t2 = (hi - p0[axis]) / d;
            t_min = t_min.max(t1.min(t2));
            t_max = t_max.min(t1.max(t2));
            if t_min > t_max {
                return None;
            }
        }
    }
    Some(t_max)
}

struct FaceProjection {
    index: usize,
    polygon: [[f32; 2]; 4],
    centroid: [f32; 2],
}

/// Every face pointing at the camera, back to front.
fn project_faces(view: &CubeView) -> Vec<FaceProjection> {
    let mut faces = Vec::with_capacity(3);
    for (index, face) in CUBE_FACES.iter().enumerate() {
        // A face is visible when its normal points toward the eye.
        let facing = dot(face.normal, view.eye);
        if facing <= 0.0 {
            continue;
        }
        let mut polygon = [[0.0f32; 2]; 4];
        let mut centroid = [0.0f32; 2];
        for (slot, corner) in face_corners(face.normal).iter().enumerate() {
            let projected = project(view, *corner);
            polygon[slot] = projected;
            centroid[0] += projected[0] * 0.25;
            centroid[1] += projected[1] * 0.25;
        }
        faces.push(FaceProjection {
            index,
            polygon,
            centroid,
        });
    }
    faces
}

fn point_in_polygon(p: [f32; 2], polygon: &[[f32; 2]; 4]) -> bool {
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let (xi, yi) = (polygon[i][0], polygon[i][1]);
        let (xj, yj) = (polygon[j][0], polygon[j][1]);
        if (yi > p[1]) != (yj > p[1]) && p[0] < (xj - xi) * (p[1] - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CubeHit {
    Face(usize),
    Edge(usize),
    Corner(usize),
}

/// The nearest hit under the pointer: a face, or an edge/corner box that pokes
/// in front of the cube's surface.
fn cube_hit(view: &CubeView, faces: &[FaceProjection], pointer: [f32; 2]) -> Option<CubeHit> {
    let sx = pointer[0] - view.center[0];
    let sy = pointer[1] - view.center[1];
    // The world point under the pointer with zero depth along the eye axis.
    let p0 = [
        view.right[0] * sx - view.up[0] * sy,
        view.right[1] * sx - view.up[1] * sy,
        view.right[2] * sx - view.up[2] * sy,
    ];
    let face_hit = faces
        .iter()
        .find(|face| point_in_polygon(pointer, &face.polygon))
        .map(|face| face.index);
    // The cube's entry depth along the eye axis (support plane of the box).
    let cube_depth = CUBE_HALF
        * (view.eye[0].abs() + view.eye[1].abs() + view.eye[2].abs());

    let mut best: Option<(f32, CubeHit)> = None;
    let mut consider = |depth: f32, hit: CubeHit| {
        // The cube's surface is at `cube_depth`; a box behind it is occluded.
        if face_hit.is_some() && depth <= cube_depth {
            return;
        }
        // Larger depth is nearer the camera.
        if best.is_none_or(|(best_depth, _)| depth > best_depth) {
            best = Some((depth, hit));
        }
    };

    for (index, direction) in EDGE_DIRS.iter().enumerate() {
        if let Some(depth) = box_front_depth(&edge_box(*direction), p0, view.eye) {
            consider(depth, CubeHit::Edge(index));
        }
    }
    for (index, direction) in CORNER_DIRS.iter().enumerate() {
        if let Some(depth) = box_front_depth(&corner_box(*direction), p0, view.eye) {
            consider(depth, CubeHit::Corner(index));
        }
    }

    best.map(|(_, hit)| hit)
        .or_else(|| face_hit.map(CubeHit::Face))
}

/// `ZUpGizmoHelper`'s home chip: a rounded button on the cube's bottom-right
/// diagonal that returns the camera to the home pose.
pub fn home_button(
    camera: Rc<std::cell::RefCell<Camera>>,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    div()
        .id("view-cube-home")
        .w(px(HOME_SIZE))
        .h(px(HOME_SIZE))
        // `roundRect(..., 16)` on a 64px canvas drawn at 28px: 7px on screen.
        .rounded(px(7.0))
        .border_1()
        .border_color(rgb(HOME_BORDER))
        .bg(rgb(HOME_CHIP))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(HOME_CHIP_HOVER)))
        .on_click(move |_, _, cx| {
            camera
                .borrow_mut()
                .home(glam::Vec3::new(0.0, 0.0, 110.0));
            entity.update(cx, |_, cx| cx.notify()).ok();
        })
        // lucide's house, the icon the rest of the UI uses. `icons::icon`
        // resolves it from the core crate's assets; a hand-built
        // `external_path` here pointed at the repo root instead.
        .child(icons::icon("house", 22.0, HOME_GLYPH))
}

/// `HOME_SIZE` from `ZUpGizmoHelper`: `0.47` of the cube's 60-unit scale.
const HOME_SIZE: f32 = 28.0;
/// `gizmoColors.chip`.
const HOME_CHIP: u32 = SURFACE_0;
/// A hover step above the chip surface.
const HOME_CHIP_HOVER: u32 = SURFACE_1;
/// `gizmoColors.chipBorder`.
const HOME_BORDER: u32 = BORDER_SUBTLE;
/// The glyph is painted in `accentColor`.
const HOME_GLYPH: u32 = 0xbaf72e;

/// The Z-up orientation cube from `ZUpGizmoViewcube.tsx`, projected and painted
/// with GPUI paths.
///
/// Clicking a face, edge or corner snaps `camera` to look straight at it and
/// notifies `entity`; hovering tints the target with the gizmo accent.
pub fn view_cube(
    camera: Rc<std::cell::RefCell<Camera>>,
    viewport: Rc<RefCell<Option<Viewport>>>,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let bounds: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));

    let bounds_for_canvas = bounds.clone();
    let paint_camera = camera.clone();
    let cube_viewport = viewport.clone();

    div()
        .id("view-cube")
        .relative()
        .w(px(CUBE_WIDGET))
        .h(px(CUBE_WIDGET))
        .child(
            canvas(
                move |canvas_bounds, _window, _cx| {
                    bounds_for_canvas.set(Some(canvas_bounds));
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .size_full(),
        )
        .child(
            gpu_canvas(move |gpu| {
                let interop = match attach(gpu.try_device::<WgpuRenderer>()) {
                    Ok(interop) => interop,
                    Err(_) => return,
                };
                let Some((device, queue)) = interop.adapter().wgpu() else {
                    return;
                };
                let mut slot = cube_viewport.borrow_mut();
                let Some(state) = slot.as_mut() else {
                    return;
                };
                let camera = paint_camera.borrow();
                state.render_cube(
                    &device,
                    &queue,
                    &camera,
                    None,
                    [0xba as f32 / 255.0, 0xf7 as f32 / 255.0, 0x2e as f32 / 255.0],
                );
                drop(camera);
                match state.cube_texture_view().to_imported_handle() {
                    Ok(handle) => {
                        gpu.paint_texture(handle, Corners::default(), 1.0, false);
                    }
                    Err(error) => eprintln!("cube: import: {error:#}"),
                }
            })
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .size_full(),
        )
        // Repaint so the hover tint follows the pointer.
        .on_mouse_move(|_, window, _| window.refresh())
        .on_click({
            let bounds = bounds.clone();
            let camera = camera.clone();
            let entity = entity.clone();
            move |event, _window, cx| {
                let Some(bounds) = bounds.get() else {
                    return;
                };
                let size = bounds.size.width.0.min(bounds.size.height.0);
                let center = [
                    bounds.origin.x.0 + size * 0.5,
                    bounds.origin.y.0 + size * 0.5,
                ];
                let position = event.position();
                let pointer = [position.x.0, position.y.0];
                let hit = {
                    let camera = camera.borrow();
                    let view = cube_view(&camera, center);
                    let faces = project_faces(&view);
                    cube_hit(&view, &faces, pointer)
                };
                let Some(hit) = hit else {
                    return;
                };
                cx.stop_propagation();
                // `tweenCamera(rotateTweenTarget(normal | position))`.
                let direction = match hit {
                    CubeHit::Face(index) => CUBE_FACES[index].normal,
                    CubeHit::Edge(index) => rotate_gizmo(EDGE_DIRS[index]),
                    CubeHit::Corner(index) => rotate_gizmo(CORNER_DIRS[index]),
                };
                let (yaw, pitch) = direction_to_yaw_pitch(direction);
                {
                    let mut camera = camera.borrow_mut();
                    camera.yaw = yaw;
                    camera.pitch = pitch;
                }
                entity.update(cx, |_, cx| cx.notify()).ok();
            }
        })
}

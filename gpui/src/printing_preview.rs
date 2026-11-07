//! `src/components/organisms/PrintingPreviewPane.tsx` — printing mode's right
//! column: the layer scrubber and the layer preview box.
//!
//! The pane is the source's two-column split: a 56px rail holding the docked,
//! embedded, compact `LayerSlider`, and a flex column with the `Layer Preview`
//! heading, the `Layer n/total` progress line and the preview area.
//!
//! Integration: the caller renders [`printing_preview`] as the last child of
//! the window root, in `Mode::Printing`. The pane positions itself absolutely
//! against the right half of the window, below the top bar, exactly as the
//! source's `absolute right-0 w-1/2` does inside `page.tsx`'s scene column.
//!
//! # PORT DEVIATIONS
//!
//! Diffed against `PrintingPreviewPane.tsx`, its `LayerSlider` /
//! `PrintingLayerGpuPreview` children, and the `page.tsx` /
//! `usePrintingPreviewManager.ts` values that feed it.
//!
//! * **GPU scrub preview** (`PrintingLayerGpuPreview`): a three.js scene, so it
//!   cannot be hosted here. Its layer is therefore absent: with a non-zero
//!   layer count and no PNG the preview box is empty, where the source would
//!   show the GPU render.
//! * **PNG layer / settled hi-res canvas** (`printingPreviewPngUrlForDisplay`,
//!   `isPrintingPngLoaded`, `selectedPrintingLayerPreviewUrl`,
//!   `usePrintingSettledHiResCanvas`, `printingPreviewCanvasRef`,
//!   `isPrintingSettledCanvasReady`): the source stacks a GPU preview under a
//!   PNG and a settled canvas over it, all keyed off the slicer's per-layer
//!   URLs. The port has no slicer, so the pane takes one already-resolved
//!   local image path ([`PrintingPreviewState::preview_image`]) and draws it
//!   with `object-fit: contain` — the same letterboxing the source's aspect box
//!   plus `object-contain` image produces. The fade-in opacities
//!   (`transition-opacity duration-100/150/75`) have no GPUI equivalent and are
//!   drawn settled.
//! * **Aspect box sizing** (`width: min(100cqw, 100cqh * ratio)`): GPUI has no
//!   container-query units and no `min()` in its layout solver, so the
//!   letterboxed box is not a laid-out element. Its dashed border — the empty
//!   state's only visible geometry — is painted onto a canvas child at the
//!   fitted, centred rect instead (see [`paint_aspect_border`]). The box's
//!   `rounded` (4px) corners are square there: `PathBuilder` has no rounded
//!   rect helper for a stroked path.
//! * **`border-dashed`**: GPUI's `border_dashed` is a layout-style field that
//!   the canvas path does not read, so the dashes are emitted as explicit
//!   subpaths — 3px dashes with 3px gaps, which is what a 1px `dashed` border
//!   renders as in Chromium.
//! * **Transforms** (`printingPreviewVisualTransform`,
//!   `printingPreviewScrubUpscaleTransform`, and the empty state's
//!   `scale(mirrorX, mirrorY)` de-mirror): GPUI has no CSS transforms, so the
//!   preview is drawn unzoomed and unflipped. The de-mirror only existed to
//!   undo the GPU preview's mirror, which is not rendered.
//! * **Preview pan / zoom** (`onWheel`, `onPointerDown/Move/Up/Cancel`,
//!   `printingPreviewCursor`): these drive the GPU/PNG preview's zoom and pan.
//!   With no interactive preview surface they are not ported, so the area
//!   keeps the platform's default cursor rather than the source's
//!   `zoom-in` / `grab` / `grabbing` / `default` states.
//! * **`containerType: 'size'`** — no GPUI counterpart; nothing reads it once
//!   the aspect box is painted rather than laid out.
//! * **`z-20` on the rail column** — this GPUI revision has no z-index; paint
//!   order follows the element tree, which is the same order here.
//! * **`tracking-wide`** on the heading — no letter-spacing in GPUI; the label
//!   is upper-cased in Rust to match the `uppercase` utility's rendering.
//! * **`printingCurrentHeightMm` / `slicingHeightMm`** — the source feeds them
//!   to the GPU preview's `clipZ` and to the slider's mm readout. Neither
//!   exists in the port, and `overlays::layer_slider` already documents the
//!   missing readout, so the pane does not take them.
//! * **`supportDragGroupRef` / `supportRenderRefreshNonce`** — three.js support
//!   geometry for the GPU preview; not applicable.
//! * **`aria-label` / `role="img"`** — GPUI elements carry no ARIA attributes.

use std::path::PathBuf;

use gpui::{
    Bounds, FontWeight, IntoElement, ObjectFit, PathBuilder, Pixels, SharedString, WeakEntity,
    Window, canvas, div, img, point, prelude::*, px, relative, rgb, rgba,
};

use crate::DragonFruit;
use crate::overlays;
use crate::theme::*;

/// `gridTemplateColumns: '56px minmax(0, 1fr)'` — the rail column.
const RAIL_COLUMN_WIDTH: f32 = 56.0;
/// `pt-1.5` on the rail column.
const RAIL_PADDING_TOP: f32 = 6.0;
/// `p-3` on the content column.
const COLUMN_PADDING: f32 = 12.0;
/// `gap-2` between the heading, the progress line and the preview area.
const COLUMN_GAP: f32 = 8.0;
/// `rounded-lg` — Tailwind v4's 8px, not `theme::RADIUS_LG`.
const PREVIEW_RADIUS: f32 = 8.0;
/// `p-2` on the preview area.
const PREVIEW_PADDING: f32 = 8.0;
/// `text-xs` — 12px with Tailwind's 1rem line-height.
const HEADING_SIZE: f32 = 12.0;
const HEADING_LINE_HEIGHT: f32 = 16.0;
/// `text-[11px]`, inheriting `body`'s `--line-height-normal`.
const PROGRESS_SIZE: f32 = 11.0;
/// `color-mix(in srgb, var(--surface-1), transparent 6%)` — the rail column and
/// the preview area share the same 94%-opaque `--surface-1`.
const SURFACE_1_94: u32 = 0x1a1b21f0;
/// `border-dashed` at 1px: Chromium's 3px dash / 3px gap.
const DASH: f32 = 3.0;
const DASH_GAP: f32 = 3.0;
/// `activePrinterProfile?.buildVolumeMm?.width ?? 143` and `?.depth ?? 89`.
const DEFAULT_BUILD_WIDTH_MM: f32 = 143.0;
const DEFAULT_BUILD_DEPTH_MM: f32 = 89.0;

/// The pane's inputs, named after the props `PrintingPreviewPane` receives.
///
/// The three.js and per-layer-URL props are not modelled — see the module's
/// `PORT DEVIATIONS`.
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

/// `PrintingPreviewPane` — the printing-mode right column.
///
/// `entity` owns the slider's value: `overlays::layer_slider` writes the upper
/// thumb into `DragonFruit::layer` and reads the cross-section switch from
/// `DragonFruit::cross_section`, so the pane and the scene clip stay in step.
///
/// The pane is absolutely positioned against the right half of the window,
/// starting below `TOPBAR_HEIGHT`; the caller only adds it to the root.
pub fn printing_preview(
    state: PrintingPreviewState,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    // `Math.max(1, Math.min(Math.max(1, total), selected))` and
    // `Math.max(1, total)`.
    let max_layer = state.total_layers.max(1);
    let selected = state.selected_layer.max(1.0).min(max_layer as f32);

    // `printingPreviewTargetResolution ? viewportWidth : buildVolumeMm.width ?? 143`
    // (and the same for the height), then `aspectW / aspectH`.
    let (aspect_w, aspect_h) = match state.viewport_mm {
        Some(viewport) => viewport,
        None => state
            .build_volume_mm
            .unwrap_or((DEFAULT_BUILD_WIDTH_MM, DEFAULT_BUILD_DEPTH_MM)),
    };
    let aspect_ratio = aspect_w / aspect_h;

    // `!selectedPrintingLayerPreviewUrl && printingPreviewTotalLayers === 0`.
    let show_empty_state = state.preview_image.is_none() && state.total_layers == 0;

    // -- rail column ---------------------------------------------------------
    // `relative z-20 h-full overflow-visible border-r px-0 pt-1.5 pb-0`, with
    // `color-mix(in srgb, var(--surface-1), transparent 6%)` behind it.
    let rail = div()
        .relative()
        .w(px(RAIL_COLUMN_WIDTH))
        .flex_shrink_0()
        .h_full()
        .border_r_1()
        .border_color(rgb(BORDER_SUBTLE))
        .px(px(0.0))
        .pt(px(RAIL_PADDING_TOP))
        .pb(px(0.0))
        .bg(rgba(SURFACE_1_94))
        .child(overlays::layer_slider(
            selected,
            None,
            state.cross_section,
            1.0,
            max_layer as f32,
            1.0,
            entity,
        ));

    // -- preview area --------------------------------------------------------
    // `relative flex-1 min-h-0 min-w-0 rounded-lg border p-2 flex items-center
    // justify-center overflow-hidden`.
    let mut preview = div()
        .relative()
        .flex_1()
        .min_h(px(0.0))
        .min_w(px(0.0))
        .rounded(px(PREVIEW_RADIUS))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .p(px(PREVIEW_PADDING))
        .flex()
        .items_center()
        .justify_center()
        .overflow_hidden()
        .bg(rgba(SURFACE_1_94));

    // The aspect box's own box, so the canvas below measures the content box
    // rather than the padded one.
    let mut content = div()
        .relative()
        .size_full()
        .flex()
        .items_center()
        .justify_center();

    if show_empty_state {
        // The empty state's dashed box, fitted to the build plate's aspect
        // ratio and centred — see the module's `PORT DEVIATIONS`.
        content = content.child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _cx| {
                    paint_aspect_border(bounds, aspect_ratio, window);
                },
            )
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .size_full(),
        );
    }

    match state.preview_image {
        // `object-contain` inside the aspect box; the box and the image share
        // the same ratio, so containing it in the content box is identical.
        Some(path) => {
            content = content.child(img(path).size_full().object_fit(ObjectFit::Contain));
        }
        None if show_empty_state => {
            content = content.child(
                div()
                    .text_size(px(HEADING_SIZE))
                    .line_height(px(HEADING_LINE_HEIGHT))
                    .text_color(rgb(TEXT_MUTED))
                    .child("No preview available yet."),
            );
        }
        None => {}
    }

    preview = preview.child(content);

    // -- content column ------------------------------------------------------
    // `h-full min-h-0 min-w-0 p-3 flex flex-col gap-2 overflow-hidden`.
    let column = div()
        .h_full()
        .min_h(px(0.0))
        .min_w(px(0.0))
        .flex_1()
        .p(px(COLUMN_PADDING))
        .flex()
        .flex_col()
        .gap(px(COLUMN_GAP))
        .overflow_hidden()
        // `text-xs font-semibold uppercase tracking-wide`, `--text-muted`.
        .child(
            div()
                .text_size(px(HEADING_SIZE))
                .line_height(px(HEADING_LINE_HEIGHT))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child("LAYER PREVIEW"),
        )
        // `formatLayerProgress`: `Layer {clamped}/{max(1, total)}`.
        .child(
            div()
                .text_size(px(PROGRESS_SIZE))
                .line_height(px(PROGRESS_SIZE * LINE_HEIGHT_NORMAL))
                .text_color(rgb(TEXT_MUTED))
                .child(SharedString::from(format!(
                    "Layer {}/{}",
                    selected.round() as i64,
                    max_layer
                ))),
        )
        .child(preview);

    // -- pane ----------------------------------------------------------------
    // `absolute right-0 w-1/2 min-w-0 grid overflow-hidden`, pinned between
    // `--topbar-height` and the bottom edge.
    div()
        .absolute()
        .right(px(0.0))
        .top(px(TOPBAR_HEIGHT))
        .bottom(px(0.0))
        .w(relative(0.5))
        .min_w(px(0.0))
        .flex()
        .overflow_hidden()
        .bg(rgb(SURFACE_0))
        .child(rail)
        .child(column)
}

/// The empty state's `border border-dashed` box, fitted to the build plate's
/// aspect ratio.
///
/// `bounds` is the preview area's content box. The box is
/// `min(content width, content height * ratio)` wide and centred, which is the
/// source's `width: min(100cqw, 100cqh * ratio)`; the border is stroked as
/// 3px dashes with 3px gaps because GPUI's path builder has no dash pattern.
fn paint_aspect_border(bounds: Bounds<Pixels>, ratio: f32, window: &mut Window) {
    let width = bounds.size.width.0;
    let height = bounds.size.height.0;
    if !(width > 0.0) || !(height > 0.0) || !ratio.is_finite() || ratio <= 0.0 {
        return;
    }

    let (box_w, box_h) = if width / height > ratio {
        (height * ratio, height)
    } else {
        (width, width / ratio)
    };
    let left = bounds.origin.x.0 + (width - box_w) * 0.5;
    let top = bounds.origin.y.0 + (height - box_h) * 0.5;
    let right = left + box_w;
    let bottom = top + box_h;

    let mut builder = PathBuilder::stroke(px(1.0));
    dash_edge(&mut builder, (left, top), (right, top));
    dash_edge(&mut builder, (right, top), (right, bottom));
    dash_edge(&mut builder, (right, bottom), (left, bottom));
    dash_edge(&mut builder, (left, bottom), (left, top));
    if let Ok(path) = builder.build() {
        window.paint_path(path, rgb(BORDER_SUBTLE));
    }
}

/// Appends one dashed edge to `builder`.
fn dash_edge(builder: &mut PathBuilder, from: (f32, f32), to: (f32, f32)) {
    let (x0, y0) = from;
    let (x1, y1) = to;
    let dx = x1 - x0;
    let dy = y1 - y0;
    let length = (dx * dx + dy * dy).sqrt();
    if !(length > 0.0) {
        return;
    }
    let (ux, uy) = (dx / length, dy / length);

    let mut start = 0.0;
    while start < length {
        let end = (start + DASH).min(length);
        builder.move_to(point(px(x0 + ux * start), px(y0 + uy * start)));
        builder.line_to(point(px(x0 + ux * end), px(y0 + uy * end)));
        start = end + DASH_GAP;
    }
}

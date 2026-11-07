//! `SliceMetricsDebugModal.tsx` — the slice-metrics sheet mounted by
//! `DiagnosticsModals.tsx`.

use std::rc::Rc;

use gpui::{
    App, ClickEvent, Div, FontWeight, PathBuilder, SharedString, Window, canvas, div, point,
    prelude::*, px, relative, rgb, rgba,
};
use gpui_core::icons::icon;
use gpui_core::theme::*;
use gpui_ui::panel_fields::{BLACK, with_alpha};

use super::chrome::{AMBER_500, LEADING_TIGHT, debug_sheet, mix_b};
use super::format::{fixed, grouped, grouped_option, num};

// ── SliceMetricsDebugModal ──────────────────────────────────────────────────

/// `NativeSlicerPerfMetrics`.
pub struct NativePerfMetrics {
    pub total_ns: Option<f64>,
    pub index_build_ns: Option<f64>,
    pub render_wall_ns: Option<f64>,
    pub render_ns: Option<f64>,
    pub png_encode_ns: Option<f64>,
    pub archive_encode_ns: Option<f64>,
    pub z_blend_backward_ns: Option<f64>,
    pub z_blend_forward_ns: Option<f64>,
    pub cross_blend_ns: Option<f64>,
    pub cross_blend_touched_pixels: Option<f64>,
    pub cross_blend_contributing_layers: Option<f64>,
    pub post_blur_ns: Option<f64>,
    pub support_merge_ns: Option<f64>,
    pub layers: Option<f64>,
}

/// `NativeSlicerRuntimeMetrics`.
pub struct NativeRuntimeMetrics {
    pub pool_threads: u32,
    pub max_concurrent: u32,
    pub queue_buffer: u32,
    pub daa_post_threads: Option<u32>,
    pub daa_post_buffer_depth: Option<u32>,
    pub build_profile: Option<SharedString>,
    pub artifact_dir: Option<SharedString>,
    pub mesh_stage_dir: Option<SharedString>,
    pub metadata_parse_ns: Option<f64>,
    pub mesh_decode_ns: Option<f64>,
    pub artifact_metadata_ns: Option<f64>,
    pub wrapper_total_ns: Option<f64>,
    pub wrapper_overhead_ns: Option<f64>,
}

/// The `jobConfig` block the dialog reads.
pub struct SliceJobConfig {
    pub output_display_name: SharedString,
    pub output_format: SharedString,
    pub source_width_px: f64,
    pub source_height_px: f64,
    pub width_px: f64,
    pub height_px: f64,
    pub build_width_mm: f64,
    pub build_depth_mm: f64,
    pub layer_height_mm: f64,
    pub anti_aliasing_level: SharedString,
    pub png_compression_strategy: SharedString,
    pub container_compression_level: f64,
    pub x_packing_mode: SharedString,
    pub mesh_transfer_mode: SharedString,
    pub mesh_encoding: SharedString,
    pub initial_mesh_staging_bytes: f64,
    pub mesh_chunk_target_bytes: f64,
    pub model_triangle_count: f64,
    pub triangle_float_count: f64,
    pub metadata_json_bytes: f64,
    pub export_thumbnail_provided: bool,
    pub export_thumbnail_bytes: f64,
    pub mesh_stage_file_path: Option<SharedString>,
}

/// The `nativePerf` block the dialog reads.
pub struct SliceNativePerf {
    pub perf: Option<NativePerfMetrics>,
    pub runtime: Option<NativeRuntimeMetrics>,
    pub bridge_payload_build_ms: Option<f64>,
    pub bridge_invoke_round_trip_ms: Option<f64>,
    pub bridge_total_ms: Option<f64>,
    pub bridge_payload_chars: Option<f64>,
    pub triangle_float_count: Option<f64>,
    pub mesh_bytes_len: Option<f64>,
    pub stage_mesh_ms: Option<f64>,
    pub stage_mesh_bytes: Option<f64>,
    pub stage_mesh_chunk_count: Option<f64>,
    pub stage_mesh_avg_chunk_bytes: Option<f64>,
    pub stage_mesh_throughput_mib_per_sec: Option<f64>,
    pub stage_mesh_ack_append_ms: Option<f64>,
    pub stage_mesh_capacity_max_bytes: Option<f64>,
    pub stage_mesh_reserve_growth_events: Option<f64>,
    pub transport_overhead_ms: Option<f64>,
    pub render_wall_ms: Option<f64>,
    pub render_cpu_ms: Option<f64>,
    pub index_build_ms: Option<f64>,
    pub png_encode_cpu_ms: Option<f64>,
    pub archive_encode_ms: Option<f64>,
    pub total_ms: Option<f64>,
    pub render_wall_ms_per_layer: Option<f64>,
    pub render_cpu_ms_per_layer: Option<f64>,
    pub png_cpu_ms_per_layer: Option<f64>,
    pub total_ms_per_layer: Option<f64>,
}

/// `SliceExportResult['benchmark']`.
pub struct SliceBenchmark {
    pub total_elapsed_ms: f64,
    pub mesh_prep_ms: Option<f64>,
    pub core_slicing_ms: Option<f64>,
    pub total_layers: Option<f64>,
    pub layers_per_second: Option<f64>,
    pub job_config: SliceJobConfig,
    pub native_perf: SliceNativePerf,
}

/// `SliceMetricsDebugModal`'s props.
pub struct SliceMetricsConfig {
    pub benchmark: SliceBenchmark,
    pub output_name: Option<SharedString>,
    pub output_size_label: SharedString,
    /// `'idle' | 'copied' | 'error'` — the copy button's transient label.
    pub copy_state: SliceCopyState,
}

/// The copy button's transient state.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SliceCopyState {
    Idle,
    Copied,
    Error,
}

/// `MetricCard` — a `rounded-lg border` tile with an icon, an uppercase label
/// and a right-aligned value.
fn metric_card(
    label: &'static str,
    value: SharedString,
    icon_name: &str,
    compact: bool,
) -> Div {
    div()
        .flex_1()
        .min_w(px(160.0))
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .px(px(if compact { 10.0 } else { 12.0 }))
        .py(px(if compact { 8.0 } else { 10.0 }))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .child(
                    div()
                        .flex()
                        .min_w(px(0.0))
                        .items_center()
                        .gap(px(6.0))
                        .text_size(px(10.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child(icon(icon_name, 16.0, TEXT_MUTED))
                        .child(div().truncate().child(label)),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_size(px(if compact { 14.0 } else { 16.0 }))
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(px(if compact { 14.0 } else { 16.0 } * LEADING_TIGHT))
                        .text_color(rgb(TEXT_STRONG))
                        .child(value),
                ),
        )
}

/// `RuntimeStat` — a label/value row with the source's bottom rule.
fn runtime_stat(label: impl Into<SharedString>, value: SharedString) -> Div {
    div()
        .flex()
        .items_start()
        .gap(px(12.0))
        .py(px(4.0))
        .border_b(px(1.0))
        .border_color(rgb(mix_b(BORDER_SUBTLE, BLACK, 0.35)))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .text_size(px(12.0))
                .text_color(rgb(TEXT_MUTED))
                .child(label.into()),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_size(px(12.0))
                .line_height(px(12.0 * LEADING_TIGHT))
                .text_color(rgb(TEXT_STRONG))
                .child(value),
        )
}

/// One slice of the pipeline pie.
struct PipelineSlice {
    name: &'static str,
    ms: Option<f64>,
    pct: f64,
    color: u32,
}

/// `PipelinePie` — the `conic-gradient` ring, painted as filled wedges.
fn pipeline_pie(slices: Vec<(f64, u32)>) -> Div {
    let slices = Rc::new(slices);
    div()
        .flex_shrink_0()
        .w(px(80.0))
        .h(px(80.0))
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    let size = bounds.size.width.0.min(bounds.size.height.0);
                    if size <= 0.0 {
                        return;
                    }
                    let center = [
                        bounds.origin.x.0 + size * 0.5,
                        bounds.origin.y.0 + size * 0.5,
                    ];
                    let radius = size * 0.5;
                    let total: f64 = slices.iter().map(|(pct, _)| pct.max(0.0)).sum();
                    if total <= 0.0 {
                        return;
                    }
                    let mut cursor = 0.0f64;
                    for (pct, color) in slices.iter() {
                        let start = cursor;
                        let end = (start + pct.max(0.0)).min(100.0);
                        cursor = end;
                        if end <= start {
                            continue;
                        }
                        let mut builder = PathBuilder::fill();
                        builder.move_to(point(px(center[0]), px(center[1])));
                        let steps = 24;
                        for step in 0..=steps {
                            let t = step as f64 / steps as f64;
                            let angle = (start + (end - start) * t) / 100.0
                                * std::f64::consts::TAU
                                - std::f64::consts::FRAC_PI_2;
                            let x = center[0] + (radius * angle.cos() as f32);
                            let y = center[1] + (radius * angle.sin() as f32);
                            builder.line_to(point(px(x), px(y)));
                        }
                        builder.close();
                        if let Ok(path) = builder.build() {
                            window.paint_path(path, rgb(*color));
                        }
                    }
                },
            )
            .size_full(),
        )
        .rounded_full()
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .child(
            // The inner disc: `inset-[26%] rounded-full` on `--surface-1`.
            div()
                .absolute()
                .left(px(20.8))
                .top(px(20.8))
                .w(px(38.4))
                .h(px(38.4))
                .rounded_full()
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_1))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(10.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(TEXT_MUTED)),
        )
}

/// `ratioPercent` — `(numerator / denominator) * 100`, or `None`.
fn ratio_percent(numerator: Option<f64>, denominator: Option<f64>) -> Option<f64> {
    let (numerator, denominator) = (numerator?, denominator?);
    if !numerator.is_finite() || !denominator.is_finite() || denominator <= 0.0 {
        return None;
    }
    Some(numerator / denominator * 100.0)
}

/// `formatMs` — `—`, `N ms` or `N s`.
fn format_ms(value: Option<f64>, digits: usize) -> String {
    let Some(value) = value.filter(|value| value.is_finite()) else {
        return "—".into();
    };
    if value >= 1000.0 {
        format!("{:.2} s", value / 1000.0)
    } else {
        fixed(value, digits)
    }
}

/// `formatNs` — `N ns` with grouping.
fn format_ns(value: Option<f64>) -> String {
    let Some(value) = value.filter(|value| value.is_finite()) else {
        return "—".into();
    };
    format!("{} ns", grouped(value.round()))
}

/// `formatRate` — `N layers/s`.
fn format_rate(value: Option<f64>) -> String {
    let Some(value) = value.filter(|value| value.is_finite()) else {
        return "—".into();
    };
    if value >= 100.0 {
        format!("{} layers/s", grouped(value.round()))
    } else {
        format!("{value:.2} layers/s")
    }
}

/// `formatMiBPerSec`.
fn format_mib_per_sec(value: Option<f64>) -> String {
    match value.filter(|value| value.is_finite()) {
        Some(value) => format!("{value:.2} MiB/s"),
        None => "—".into(),
    }
}

/// `formatBytes` (slice metrics) — KiB / MiB with two decimals.
fn format_bytes_slice(value: Option<f64>) -> String {
    let Some(value) = value.filter(|value| value.is_finite()) else {
        return "—".into();
    };
    let bytes = value.max(0.0);
    if bytes < 1024.0 {
        format!("{} B", bytes.round())
    } else if bytes < 1024.0 * 1024.0 {
        format!("{:.2} KiB", bytes / 1024.0)
    } else {
        format!("{:.2} MiB", bytes / (1024.0 * 1024.0))
    }
}

/// `formatPercent` (slice metrics) — `N.N%`.
fn format_percent_slice(value: Option<f64>) -> String {
    match value.filter(|value| value.is_finite()) {
        Some(value) => format!("{value:.1}%"),
        None => "—".into(),
    }
}

/// `SliceMetricsDebugModal` — the slice performance sheet.
///
/// Backdrop `rgba(0,0,0,0.72)`, a `min(94vw,1500px)` panel capped at 92vh, the
/// sticky header with the accent `Activity` tile, `Copy` and the close tile,
/// the eight metric cards, the pipeline pie and its breakdown, the raw counters
/// and the runtime / output configuration columns.
#[allow(clippy::too_many_arguments)]
pub fn slice_metrics_debug_modal(
    config: SliceMetricsConfig,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_copy: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let benchmark = &config.benchmark;
    let native = &benchmark.native_perf;
    let perf = native.perf.as_ref();
    let runtime = native.runtime.as_ref();

    let render_wall_pct = ratio_percent(native.render_wall_ms, native.total_ms);
    let index_pct = ratio_percent(native.index_build_ms, native.total_ms);
    let archive_pct = ratio_percent(native.archive_encode_ms, native.total_ms);
    let known_wall_pct =
        index_pct.unwrap_or(0.0) + render_wall_pct.unwrap_or(0.0) + archive_pct.unwrap_or(0.0);
    let other_wall_pct = (100.0 - known_wall_pct).max(0.0);
    let other_wall_ms = native.total_ms.map(|total| {
        (total
            - (native.index_build_ms.unwrap_or(0.0)
                + native.render_wall_ms.unwrap_or(0.0)
                + native.archive_encode_ms.unwrap_or(0.0)))
        .max(0.0)
    });

    let wall_vs_native_pct = ratio_percent(Some(benchmark.total_elapsed_ms), native.total_ms);
    let core_vs_native_pct = ratio_percent(benchmark.core_slicing_ms, native.total_ms);
    let worker_cpu_aggregate_ms = {
        let sum = native.render_cpu_ms.unwrap_or(0.0) + native.png_encode_cpu_ms.unwrap_or(0.0);
        if sum == 0.0 { None } else { Some(sum) }
    };
    let worker_cpu_vs_render_wall_pct =
        ratio_percent(worker_cpu_aggregate_ms, native.render_wall_ms);
    let mesh_prep_vs_wall_pct =
        ratio_percent(benchmark.mesh_prep_ms, Some(benchmark.total_elapsed_ms));
    let core_vs_wall_pct =
        ratio_percent(benchmark.core_slicing_ms, Some(benchmark.total_elapsed_ms));
    let native_vs_wall_pct =
        ratio_percent(native.total_ms, Some(benchmark.total_elapsed_ms));
    let has_native_breakdown = native.total_ms.is_some_and(|total| total > 0.0);

    let slices = vec![
        PipelineSlice {
            name: "Index build",
            ms: native.index_build_ms,
            pct: if has_native_breakdown {
                index_pct.unwrap_or(0.0).max(0.0)
            } else {
                0.0
            },
            color: mix_b(0x60a5fa, SURFACE_1, 0.20),
        },
        PipelineSlice {
            name: "Render pipeline",
            ms: native.render_wall_ms,
            pct: if has_native_breakdown {
                render_wall_pct.unwrap_or(0.0).max(0.0)
            } else {
                0.0
            },
            color: mix(ACCENT, 0xf472b6, 0.72),
        },
        PipelineSlice {
            name: "Archive encode",
            ms: native.archive_encode_ms,
            pct: if has_native_breakdown {
                archive_pct.unwrap_or(0.0).max(0.0)
            } else {
                0.0
            },
            color: mix_b(AMBER_500, SURFACE_1, 0.14),
        },
        PipelineSlice {
            name: "Other / overhead",
            ms: other_wall_ms,
            pct: if has_native_breakdown {
                other_wall_pct.max(0.0)
            } else {
                0.0
            },
            color: mix_b(0x94a3b8, SURFACE_1, 0.18),
        },
    ];

    let triangles_per_layer = match benchmark.total_layers {
        Some(layers) if layers > 0.0 => Some(benchmark.job_config.triangle_float_count / 9.0 / layers),
        _ => None,
    };

    let copy_label: &'static str = match config.copy_state {
        SliceCopyState::Idle => "Copy",
        SliceCopyState::Copied => "Copied!",
        SliceCopyState::Error => "Copy failed",
    };
    let copy_bg = if config.copy_state == SliceCopyState::Copied {
        mix_b(ACCENT, SURFACE_1, 0.88)
    } else {
        SURFACE_1
    };
    let copy_color = if config.copy_state == SliceCopyState::Copied {
        TEXT_STRONG
    } else {
        TEXT_MUTED
    };

    let header = div()
        .flex()
        .items_center()
        .justify_between()
        .p(px(16.0))
        .border_b(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .child(
            div()
                .flex()
                .min_w(px(0.0))
                .items_center()
                .gap(px(12.0))
                .child(
                    div()
                        .flex()
                        .flex_shrink_0()
                        .items_center()
                        .justify_center()
                        .w(px(40.0))
                        .h(px(40.0))
                        .rounded(px(ROUNDED_LG))
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .bg(rgb(mix_b(ACCENT, SURFACE_1, 0.88)))
                        .child(icon("activity", 20.0, ACCENT)),
                )
                .child(
                    div()
                        .min_w(px(0.0))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(18.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Slice Performance Metrics (V3)"),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(px(12.0))
                                .text_color(rgb(TEXT_MUTED))
                                .child(format!(
                                    "{} • {}",
                                    config
                                        .output_name
                                        .clone()
                                        .unwrap_or_else(|| "Latest slicing run".into()),
                                    config.output_size_label
                                )),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .id("slice-metrics-copy")
                        .flex()
                        .items_center()
                        .justify_center()
                        .h(px(36.0))
                        .rounded(px(ROUNDED_LG))
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .bg(rgb(copy_bg))
                        .px(px(12.0))
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(copy_color))
                        .cursor_pointer()
                        .on_click(on_copy)
                        .child(copy_label),
                )
                .child(
                    div()
                        .id("slice-metrics-close")
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(36.0))
                        .h(px(36.0))
                        .rounded(px(ROUNDED_LG))
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .cursor_pointer()
                        .hover(|style| style.bg(rgba(with_alpha(0xef4444, 0x1a))))
                        .on_click(on_close)
                        .child(icon("x", 16.0, TEXT_MUTED)),
                ),
        );

    // ── metric cards ───────────────────────────────────────────────────────
    let primary_cards = div()
        .flex()
        .flex_wrap()
        .gap(px(10.0))
        .child(metric_card(
            "Total wall time",
            format_ms(Some(benchmark.total_elapsed_ms), 2).into(),
            "timer",
            false,
        ))
        .child(metric_card(
            "Core slicing",
            format_ms(benchmark.core_slicing_ms, 2).into(),
            "cpu",
            false,
        ))
        .child(metric_card(
            "Native total",
            format_ms(native.total_ms, 2).into(),
            "activity",
            false,
        ))
        .child(metric_card(
            "Throughput",
            format_rate(benchmark.layers_per_second).into(),
            "activity",
            false,
        ));

    let secondary_cards = div()
        .flex()
        .flex_wrap()
        .gap(px(8.0))
        .child(metric_card(
            "Total layers",
            benchmark
                .total_layers
                .map(|layers| grouped(layers))
                .unwrap_or_else(|| "—".into())
                .into(),
            "layers",
            true,
        ))
        .child(metric_card(
            "Render wall share",
            format_percent_slice(render_wall_pct).into(),
            "activity",
            true,
        ))
        .child(metric_card(
            "Native vs wall",
            format_percent_slice(native_vs_wall_pct).into(),
            "activity",
            true,
        ))
        .child(metric_card(
            "Core vs native",
            format_percent_slice(core_vs_native_pct).into(),
            "activity",
            true,
        ));

    // ── pipeline timing ────────────────────────────────────────────────────
    let mut slice_stats = div().flex().flex_col().gap(px(2.0)).flex_1().min_w(px(0.0));
    for slice in &slices {
        slice_stats = slice_stats.child(runtime_stat(
            slice.name,
            format!(
                "{} • {}",
                format_ms(slice.ms, 3),
                format_percent_slice(Some(slice.pct))
            )
            .into(),
        ));
    }
    slice_stats = slice_stats.child(runtime_stat(
        "Native total",
        format!(
            "{} • {}",
            format_ms(native.total_ms, 3),
            format_percent_slice(if has_native_breakdown { Some(100.0) } else { None })
        )
        .into(),
    ));

    let pie_slices: Vec<(f64, u32)> = slices.iter().map(|slice| (slice.pct, slice.color)).collect();
    let pie_card = div()
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(10.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(12.0))
                .child(pipeline_pie(pie_slices))
                .child(slice_stats),
        );

    let per_layer_card = div()
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(10.0))
        .child(
            div()
                .mb(px(6.0))
                .text_size(px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child("Per-layer KPIs"),
        )
        .child(runtime_stat(
            "Render wall / layer",
            format_ms(native.render_wall_ms_per_layer, 3).into(),
        ))
        .child(runtime_stat(
            "Render CPU / layer",
            format_ms(native.render_cpu_ms_per_layer, 3).into(),
        ))
        .child(runtime_stat(
            "PNG CPU / layer",
            format_ms(native.png_cpu_ms_per_layer, 3).into(),
        ))
        .child(runtime_stat(
            "Native total / layer",
            format_ms(native.total_ms_per_layer, 3).into(),
        ));

    let overhead_card = div()
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(10.0))
        .child(
            div()
                .mb(px(6.0))
                .text_size(px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child("Overhead Snapshot"),
        )
        .child(runtime_stat(
            "Wall vs native total",
            format_percent_slice(wall_vs_native_pct).into(),
        ))
        .child(runtime_stat(
            "Native vs app wall",
            format_percent_slice(native_vs_wall_pct).into(),
        ))
        .child(runtime_stat(
            "Core vs app wall",
            format_percent_slice(core_vs_wall_pct).into(),
        ))
        .child(runtime_stat(
            "Mesh prep vs app wall",
            format_percent_slice(mesh_prep_vs_wall_pct).into(),
        ))
        .child(runtime_stat(
            "CPU agg vs render wall",
            format_percent_slice(worker_cpu_vs_render_wall_pct).into(),
        ))
        .child(runtime_stat(
            "Transport overhead",
            format_ms(native.transport_overhead_ms, 2).into(),
        ))
        .child(runtime_stat(
            "Stage mesh IPC",
            format_ms(native.stage_mesh_ms, 2).into(),
        ))
        .child(runtime_stat(
            "Stage throughput",
            format_mib_per_sec(native.stage_mesh_throughput_mib_per_sec).into(),
        ));

    let pipeline_card = div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(12.0))
        .child(
            div()
                .mb(px(8.0))
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child("Pipeline Timing"),
        )
        .child(pie_card)
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(12.0))
                .child(div().flex_1().min_w(px(240.0)).child(per_layer_card))
                .child(div().flex_1().min_w(px(240.0)).child(overhead_card)),
        );

    // ── raw counters ───────────────────────────────────────────────────────
    let mut counters = div()
        .flex()
        .flex_wrap()
        .gap(px(0.0))
        .text_size(px(12.0));
    let counter_rows: Vec<(&'static str, String)> = vec![
        (
            "Stage file path",
            benchmark
                .job_config
                .mesh_stage_file_path
                .clone()
                .map(|path| path.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
        (
            "artifactDir",
            runtime
                .and_then(|runtime| runtime.artifact_dir.clone())
                .map(|dir| dir.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
        (
            "meshStageDir",
            runtime
                .and_then(|runtime| runtime.mesh_stage_dir.clone())
                .map(|dir| dir.to_string())
                .unwrap_or_else(|| "—".into()),
        ),
        (
            "bridgePayloadChars",
            grouped_option(native.bridge_payload_chars),
        ),
        (
            "triangleFloatCount",
            grouped_option(native.triangle_float_count),
        ),
        ("meshBytesLen", grouped_option(native.mesh_bytes_len)),
        (
            "stageMeshChunkCount",
            grouped_option(native.stage_mesh_chunk_count),
        ),
        (
            "stageMeshAvgChunkBytes",
            grouped_option(native.stage_mesh_avg_chunk_bytes),
        ),
        (
            "stageMeshAckAppendMs",
            native
                .stage_mesh_ack_append_ms
                .map(|value| format!("{value:.3}"))
                .unwrap_or_else(|| "—".into()),
        ),
        (
            "stageMeshCapacityMaxBytes",
            grouped_option(native.stage_mesh_capacity_max_bytes),
        ),
        (
            "stageMeshReserveGrowthEvents",
            grouped_option(native.stage_mesh_reserve_growth_events),
        ),
        ("totalNs", format_ns(perf.and_then(|perf| perf.total_ns))),
        (
            "indexBuildNs",
            format_ns(perf.and_then(|perf| perf.index_build_ns)),
        ),
        (
            "renderWallNs",
            format_ns(perf.and_then(|perf| perf.render_wall_ns)),
        ),
        ("renderNs", format_ns(perf.and_then(|perf| perf.render_ns))),
        (
            "pngEncodeNs",
            format_ns(perf.and_then(|perf| perf.png_encode_ns)),
        ),
        (
            "archiveEncodeNs",
            format_ns(perf.and_then(|perf| perf.archive_encode_ns)),
        ),
        (
            "zBlendBackwardNs",
            format_ns(perf.and_then(|perf| perf.z_blend_backward_ns)),
        ),
        (
            "zBlendForwardNs",
            format_ns(perf.and_then(|perf| perf.z_blend_forward_ns)),
        ),
        (
            "crossBlendNs",
            format_ns(perf.and_then(|perf| perf.cross_blend_ns)),
        ),
        (
            "crossBlendTouchedPixels",
            grouped_option(perf.and_then(|perf| perf.cross_blend_touched_pixels)),
        ),
        (
            "crossBlendContributingLayers",
            grouped_option(perf.and_then(|perf| perf.cross_blend_contributing_layers)),
        ),
        (
            "postBlurNs",
            format_ns(perf.and_then(|perf| perf.post_blur_ns)),
        ),
        (
            "supportMergeNs",
            format_ns(perf.and_then(|perf| perf.support_merge_ns)),
        ),
        (
            "layers",
            perf.and_then(|perf| perf.layers)
                .map(|layers| format!("{layers}"))
                .unwrap_or_else(|| "—".into()),
        ),
        (
            "Core vs native total",
            format_percent_slice(core_vs_native_pct),
        ),
        (
            "Bridge payload build",
            format_ms(native.bridge_payload_build_ms, 2),
        ),
        (
            "Bridge invoke roundtrip",
            format_ms(native.bridge_invoke_round_trip_ms, 2),
        ),
        ("Bridge total", format_ms(native.bridge_total_ms, 2)),
        (
            "Worker CPU aggregate",
            format_ms(worker_cpu_aggregate_ms, 2),
        ),
        ("Mesh prep", format_ms(benchmark.mesh_prep_ms, 2)),
    ];
    for (label, value) in counter_rows {
        counters = counters.child(
            div()
                .w(relative(0.5))
                .min_w(px(240.0))
                .child(runtime_stat(label, value.into())),
        );
    }

    let raw_card = div()
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(12.0))
        .child(
            div()
                .mb(px(8.0))
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child("Raw Counters"),
        )
        .child(
            div()
                .id("slice-metrics-counters")
                .max_h(relative(0.28))
                .overflow_y_scroll()
                .child(counters),
        );

    // ── runtime / output configuration ─────────────────────────────────────
    let runtime_card = div()
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(12.0))
        .child(
            div()
                .mb(px(8.0))
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child("V3 Runtime Configuration"),
        )
        .child(runtime_stat(
            "Rayon pool threads",
            runtime
                .map(|runtime| runtime.pool_threads.to_string())
                .unwrap_or_else(|| "—".into())
                .into(),
        ))
        .child(runtime_stat(
            "Max concurrent workers",
            runtime
                .map(|runtime| runtime.max_concurrent.to_string())
                .unwrap_or_else(|| "—".into())
                .into(),
        ))
        .child(runtime_stat(
            "Bounded queue buffer",
            runtime
                .map(|runtime| runtime.queue_buffer.to_string())
                .unwrap_or_else(|| "—".into())
                .into(),
        ))
        .child(runtime_stat(
            "3DAA post threads",
            runtime
                .and_then(|runtime| runtime.daa_post_threads)
                .map(|threads| threads.to_string())
                .unwrap_or_else(|| "—".into())
                .into(),
        ))
        .child(runtime_stat(
            "3DAA post buffer",
            runtime
                .and_then(|runtime| runtime.daa_post_buffer_depth)
                .map(|depth| depth.to_string())
                .unwrap_or_else(|| "—".into())
                .into(),
        ))
        .child(runtime_stat(
            "Build profile",
            runtime
                .and_then(|runtime| runtime.build_profile.clone())
                .unwrap_or_else(|| "—".into()),
        ))
        .child(runtime_stat(
            "Metadata parse",
            format_ns(runtime.and_then(|runtime| runtime.metadata_parse_ns)).into(),
        ))
        .child(runtime_stat(
            "Mesh decode",
            format_ns(runtime.and_then(|runtime| runtime.mesh_decode_ns)).into(),
        ))
        .child(runtime_stat(
            "Artifact metadata",
            format_ns(runtime.and_then(|runtime| runtime.artifact_metadata_ns)).into(),
        ))
        .child(runtime_stat(
            "Wrapper total",
            format_ns(runtime.and_then(|runtime| runtime.wrapper_total_ns)).into(),
        ))
        .child(runtime_stat(
            "Wrapper overhead",
            format_ns(runtime.and_then(|runtime| runtime.wrapper_overhead_ns)).into(),
        ));

    let job = &benchmark.job_config;
    let output_card = div()
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(12.0))
        .child(
            div()
                .mb(px(8.0))
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child("Output & Render Configuration"),
        )
        .child(runtime_stat(
            "Output format",
            format!("{} ({})", job.output_display_name, job.output_format).into(),
        ))
        .child(runtime_stat(
            "Source raster",
            format!("{} × {}", num(job.source_width_px), num(job.source_height_px)).into(),
        ))
        .child(runtime_stat(
            "Logical output",
            format!("{} × {}", num(job.width_px), num(job.height_px)).into(),
        ))
        .child(runtime_stat(
            "Build area",
            format!(
                "{:.2} × {:.2} mm",
                job.build_width_mm, job.build_depth_mm
            )
            .into(),
        ))
        .child(runtime_stat(
            "Layer height",
            format!("{:.4} mm", job.layer_height_mm).into(),
        ))
        .child(runtime_stat("AA level", job.anti_aliasing_level.clone()))
        .child(runtime_stat(
            "PNG strategy",
            job.png_compression_strategy.clone(),
        ))
        .child(runtime_stat(
            "Container compression",
            num(job.container_compression_level).into(),
        ))
        .child(runtime_stat("X packing mode", job.x_packing_mode.clone()))
        .child(runtime_stat(
            "Mesh transfer mode",
            job.mesh_transfer_mode.clone(),
        ))
        .child(runtime_stat("Mesh encoding", job.mesh_encoding.clone()))
        .child(runtime_stat(
            "Initial staging reserve",
            format_bytes_slice(Some(job.initial_mesh_staging_bytes)).into(),
        ))
        .child(runtime_stat(
            "Target chunk size",
            format_bytes_slice(Some(job.mesh_chunk_target_bytes)).into(),
        ));

    let geometry_card = div()
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(12.0))
        .child(
            div()
                .mb(px(8.0))
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child("Geometry & Payload"),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .child(
                    div()
                        .w(relative(0.5))
                        .min_w(px(240.0))
                        .child(runtime_stat(
                            "Model triangles",
                            grouped(job.model_triangle_count).into(),
                        ))
                        .child(runtime_stat(
                            "Triangles / layer",
                            triangles_per_layer
                                .map(|value| format!("{value:.2}"))
                                .unwrap_or_else(|| "—".into())
                                .into(),
                        ))
                        .child(runtime_stat(
                            "Triangle floats",
                            grouped(job.triangle_float_count).into(),
                        ))
                        .child(runtime_stat(
                            "Staged mesh bytes",
                            format_bytes_slice(native.stage_mesh_bytes).into(),
                        ))
                        .child(runtime_stat(
                            "Mesh payload bytes",
                            format_bytes_slice(native.mesh_bytes_len).into(),
                        ))
                        .child(runtime_stat(
                            "Bridge payload chars",
                            grouped_option(native.bridge_payload_chars).into(),
                        )),
                )
                .child(
                    div()
                        .w(relative(0.5))
                        .min_w(px(240.0))
                        .child(runtime_stat(
                            "Staged chunk count",
                            grouped_option(native.stage_mesh_chunk_count).into(),
                        ))
                        .child(runtime_stat(
                            "Average chunk bytes",
                            format_bytes_slice(native.stage_mesh_avg_chunk_bytes).into(),
                        ))
                        .child(runtime_stat(
                            "Staging max capacity",
                            format_bytes_slice(native.stage_mesh_capacity_max_bytes).into(),
                        ))
                        .child(runtime_stat(
                            "Reserve growth events",
                            grouped(native.stage_mesh_reserve_growth_events.unwrap_or(0.0)).into(),
                        ))
                        .child(runtime_stat(
                            "Metadata JSON",
                            format_bytes_slice(Some(job.metadata_json_bytes)).into(),
                        ))
                        .child(runtime_stat(
                            "Thumbnail",
                            if job.export_thumbnail_provided {
                                format_bytes_slice(Some(job.export_thumbnail_bytes))
                            } else {
                                "none".into()
                            }
                            .into(),
                        )),
                ),
        );

    let body = div()
        .id("slice-metrics-body")
        .flex()
        .flex_col()
        .gap(px(16.0))
        .p(px(16.0))
        .child(primary_cards)
        .child(secondary_cards)
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(12.0))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(420.0))
                        .flex()
                        .flex_col()
                        .gap(px(12.0))
                        .child(pipeline_card)
                        .child(raw_card),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(300.0))
                        .flex()
                        .flex_col()
                        .gap(px(12.0))
                        .child(runtime_card)
                        .child(output_card),
                ),
        )
        .child(geometry_card);

    debug_sheet(1500.0, 0xb8, 0.0, |panel| {
        panel
            .id("slice-metrics-panel")
            .relative()
            .max_h(relative(0.92))
            .overflow_y_scroll()
            .child(header)
            .child(body)
    })
}


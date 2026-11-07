//! `DiagnosticsModals.tsx` and `DiagnosticsModal.tsx`.

use std::rc::Rc;

use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, PathBuilder, SharedString, Window, canvas, div,
    point, prelude::*, px, relative, rgb, rgba,
};
use gpui_core::theme::*;
use gpui_ui::panel_fields::{BLACK, WHITE, with_alpha};
use gpui_ui::ui::{IconButtonSize, IconButtonVariant, SelectOption, icon_button, select_dropdown};

use super::chrome::{
    Run, debug_sheet, mix_b, shadow, table_cell, table_header_cell,
};
use super::format::{fixed, quantity, grouped, format_bytes, format_percent};

// ── Diagnostics ─────────────────────────────────────────────────────────────

/// `SupportDiagnosticsStats` — the per-collection counts plus the segment,
/// joint, primitive and GPU-picking totals.
pub struct SupportDiagnosticsStats {
    /// `SUPPORT_COLLECTION_KEYS` in registry order with their counts.
    pub collections: Vec<(SharedString, usize)>,
    pub segment_count: usize,
    pub joint_count: usize,
    pub estimated_renderable_primitives: usize,
    pub picking_registered_total: usize,
    pub picking_registered_support_related: usize,
    pub picking_visible_objects: usize,
    pub picking_cached_objects: usize,
    pub picking_avg_ms: f64,
    pub picking_last_ms: f64,
    pub picking_sync_avg_ms: f64,
    pub picking_picks_per_second: f64,
}

/// `RuntimeStats` — the sampled telemetry the dialog renders.
pub struct DiagnosticsStats {
    pub fps: f64,
    pub frame_time_ms: f64,
    pub cpu_usage_estimate: f64,
    pub fps_history: Vec<f64>,
    pub frame_time_history: Vec<f64>,
    pub used_js_heap_bytes: Option<f64>,
    pub total_js_heap_bytes: Option<f64>,
    pub heap_limit_bytes: Option<f64>,
    pub webgl_mode: SharedString,
    pub support_stats: SupportDiagnosticsStats,
}

/// `DiagnosticsBenchmarkStressProfile`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BenchmarkStressProfile {
    Quick,
    Standard,
    Torture,
}

impl BenchmarkStressProfile {
    fn label(self) -> &'static str {
        match self {
            Self::Quick => "Quick",
            Self::Standard => "Standard",
            Self::Torture => "Torture",
        }
    }

    fn value(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Standard => "standard",
            Self::Torture => "torture",
        }
    }
}

/// `benchmarkRunState`.
pub struct BenchmarkRunState {
    pub is_running: bool,
    pub phase_label: SharedString,
    pub message: SharedString,
    pub error: Option<SharedString>,
}

/// One phase row of the benchmark table.
pub struct BenchmarkPhaseStats {
    pub fps_avg: f64,
    pub fps_min: f64,
    pub frame_time_avg_ms: f64,
    pub frame_time_p95_ms: f64,
    pub frame_time_max_ms: f64,
}

/// One benchmark phase.
pub struct BenchmarkPhase {
    pub phase: SharedString,
    pub stats: BenchmarkPhaseStats,
}

/// `DiagnosticsBenchmarkResult`.
pub struct DiagnosticsBenchmarkResult {
    pub fps_avg: f64,
    pub frame_time_avg_ms: f64,
    pub frame_time_p95_ms: f64,
    pub projection_mode: SharedString,
    pub total_duration_ms: f64,
    pub camera_feel_preset: SharedString,
    pub stress_profile: SharedString,
    pub phases: Vec<BenchmarkPhase>,
}

/// `DiagnosticsModal`'s scene and system facts.
pub struct DiagnosticsSceneFacts {
    pub app_mode: SharedString,
    pub camera_projection_mode: SharedString,
    pub model_count: usize,
    pub visible_model_count: usize,
    pub selected_model_count: usize,
    pub total_polygons: f64,
    pub selected_polygons: f64,
    pub logical_cpu_cores: Option<usize>,
    pub device_memory_gb: Option<f64>,
    pub user_agent: SharedString,
}

/// `Sparkline` — a 2px `PathBuilder` polyline over the source's 380×88
/// viewBox, stretched to the box (`preserveAspectRatio="none"`).
fn sparkline(values: Vec<f64>, min: f64, max: f64, stroke: u32) -> Div {
    let values = Rc::new(values);
    div().w_full().h(px(96.0)).child(
        canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                let width = bounds.size.width.0;
                let height = bounds.size.height.0;
                let y_range = (max - min).max(1e-6);
                let count = values.len();
                if count == 0 {
                    return;
                }
                let mut builder = PathBuilder::stroke(px(2.0));
                for (index, value) in values.iter().enumerate() {
                    let x = (index as f32 / (count.max(2) - 1) as f32) * width;
                    let y = height - (((value - min) / y_range) as f32) * height;
                    let y = y.clamp(0.0, height);
                    if index == 0 {
                        builder.move_to(point(px(x), px(y)));
                    } else {
                        builder.line_to(point(px(x), px(y)));
                    }
                }
                if let Ok(path) = builder.build() {
                    window.paint_path(path, rgb(stroke));
                }
            },
        )
        .size_full(),
    )
}

/// A `rounded-lg border p-3` stat card on `--surface-1`.
fn stat_card() -> Div {
    div()
        .flex_1()
        .min_w(px(150.0))
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(12.0))
}

/// One `text-[11px]` label over a `text-xl font-semibold` value.
fn stat_tile(label: &'static str, value: SharedString) -> Div {
    stat_card()
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(TEXT_MUTED))
                .child(label),
        )
        .child(
            div()
                .text_size(px(22.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(value),
        )
}

/// One `Scene Stats` / `System` / `Support Stats` line: a muted label and a
/// strong value.
fn stat_line(label: impl Into<SharedString>, value: SharedString) -> Div {
    div()
        .flex()
        .items_baseline()
        .gap(px(4.0))
        .child(
            div()
                .text_color(rgb(TEXT_MUTED))
                .child(label.into()),
        )
        .child(
            div()
                .text_color(rgb(TEXT_STRONG))
                .child(value),
        )
}

/// `DiagnosticsModal` — the runtime telemetry sheet.
///
/// Backdrop `bg-black/60 px-4`, a `max-w-5xl` panel with the
/// `0 28px 64px rgba(0,0,0,0.48)` shadow, the `Diagnostics` header with
/// `Ctrl+Shift+D` hint, the eight stat tiles, the orbit benchmark block, the
/// two sparklines and the scene / system / support / picking columns.
#[allow(clippy::too_many_arguments)]
pub fn diagnostics_modal(
    stats: DiagnosticsStats,
    facts: DiagnosticsSceneFacts,
    benchmark_result: Option<DiagnosticsBenchmarkResult>,
    stress_profile: BenchmarkStressProfile,
    run_state: BenchmarkRunState,
    profile_open: bool,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_toggle_profile: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_profile: impl Fn(usize, &mut Window, &mut App) + 'static,
    on_run_benchmark: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let heap_usage_ratio = match (stats.used_js_heap_bytes, stats.total_js_heap_bytes) {
        (Some(used), Some(total)) => Some(used / total.max(1.0)),
        _ => None,
    };

    let header = div()
        .flex()
        .items_center()
        .justify_between()
        .border_b(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
        .px(px(16.0))
        .py(px(12.0))
        .child(
            div()
                .child(
                    div()
                        .text_size(px(16.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Diagnostics"),
                )
                .child(
                    div()
                        .mt(px(2.0))
                        .text_size(px(11.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Runtime telemetry for DragonFruit (toggle with Ctrl+Shift+D)"),
                ),
        )
        .child(
            icon_button(
                ElementId::named_usize("diagnostics", 0),
                "x",
                IconButtonVariant::Surface,
                IconButtonSize::Sm,
            )
            .on_click(on_close),
        );

    // ── stat tiles ─────────────────────────────────────────────────────────
    let tiles = div()
        .flex()
        .flex_wrap()
        .gap(px(12.0))
        .child(stat_tile("FPS", fixed(stats.fps, 1).into()))
        .child(stat_tile(
            "Frame Time",
            quantity(stats.frame_time_ms, 2, "ms").into(),
        ))
        .child(stat_tile(
            "Used JS Heap",
            format_bytes(stats.used_js_heap_bytes).into(),
        ))
        .child(stat_tile(
            "Heap Usage",
            format_percent(heap_usage_ratio).into(),
        ))
        .child(stat_tile(
            "CPU Usage (est.)",
            quantity(stats.cpu_usage_estimate, 0, "percent").into(),
        ))
        .child(stat_tile(
            "Support Segments",
            grouped(stats.support_stats.segment_count as f64).into(),
        ))
        .child(stat_tile(
            "Support Pickers",
            grouped(stats.support_stats.picking_registered_support_related as f64).into(),
        ))
        .child(stat_tile(
            "Picking Avg",
            quantity(stats.support_stats.picking_avg_ms, 3, "ms").into(),
        ));

    // ── benchmark block ────────────────────────────────────────────────────
    let profile_options: Vec<SelectOption> = [
        BenchmarkStressProfile::Quick,
        BenchmarkStressProfile::Standard,
        BenchmarkStressProfile::Torture,
    ]
    .iter()
    .map(|profile| SelectOption::new(profile.value(), profile.label()))
    .collect();

    let run_button = {
        let (border, background, color) = if run_state.is_running {
            (BORDER_SUBTLE, SURFACE_2, TEXT_MUTED)
        } else {
            (
                mix_b(ACCENT, WHITE, 0.10),
                mix_b(ACCENT, SURFACE_0, 0.76),
                ACCENT_CONTRAST,
            )
        };
        let mut el = div()
            .id("diagnostics-run-benchmark")
            .flex()
            .items_center()
            .justify_center()
            .h(px(36.0))
            .rounded(px(ROUNDED_MD))
            .border_1()
            .border_color(rgb(border))
            .bg(rgb(background))
            .px(px(12.0))
            .text_size(px(12.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(color))
            .child(if run_state.is_running {
                "Running…"
            } else {
                "Run Benchmark"
            });
        if run_state.is_running {
            el = el.opacity(0.45).cursor_not_allowed();
        } else {
            el = el
                .cursor_pointer()
                .on_click(move |event, window, cx| on_run_benchmark(event, window, cx));
        }
        el
    };

    let mut benchmark_block = div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(12.0))
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .child(
                    div()
                        .child(
                            div()
                                .text_size(px(12.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child("Automated Orbit Benchmark"),
                        )
                        .child(
                            div()
                                .mt(px(2.0))
                                .text_size(px(11.0))
                                .text_color(rgb(TEXT_MUTED))
                                .child(
                                    "Runs slow, medium, and fast full 3D sweep passes around the \
                                     current model and records frame timing.",
                                ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div().w(px(118.0)).child(select_dropdown(
                                "diagnostics-stress-profile",
                                &profile_options,
                                stress_profile.value(),
                                profile_open,
                                run_state.is_running,
                                move |event, window, cx| on_toggle_profile(event, window, cx),
                                move |index, window, cx| on_select_profile(index, window, cx),
                            )),
                        )
                        .child(run_button),
                ),
        )
        .child(
            div()
                .mt(px(8.0))
                .text_size(px(11.0))
                .text_color(rgb(if run_state.error.is_some() {
                    DANGER
                } else {
                    TEXT_MUTED
                }))
                .child({
                    let base: SharedString = match &run_state.error {
                        Some(error) => error.clone(),
                        None => {
                            if run_state.message.is_empty() {
                                "No benchmark run yet.".into()
                            } else {
                                run_state.message.clone()
                            }
                        }
                    };
                    if run_state.phase_label.is_empty() {
                        base
                    } else {
                        format!("{base} • {}", run_state.phase_label).into()
                    }
                }),
        );

    if let Some(result) = &benchmark_result {
        let mut phases_table = div()
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(px(ROUNDED_MD))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_0));
        let head = div()
            .flex()
            .bg(rgb(SURFACE_0))
            .text_color(rgb(TEXT_MUTED))
            .text_size(px(11.0))
            .child(table_header_cell("Phase"))
            .child(table_header_cell("Avg FPS"))
            .child(table_header_cell("Min FPS"))
            .child(table_header_cell("Avg FT"))
            .child(table_header_cell("P95 FT"))
            .child(table_header_cell("Max FT"));
        phases_table = phases_table.child(head);
        for phase in &result.phases {
            phases_table = phases_table.child(
                div()
                    .flex()
                    .border_t(px(1.0))
                    .border_color(rgb(BORDER_SUBTLE))
                    .bg(rgb(SURFACE_1))
                    .text_size(px(11.0))
                    .child(table_cell(
                        div()
                            .text_color(rgb(TEXT_STRONG))
                            .child(phase.phase.clone()),
                    ))
                    .child(table_cell(
                        div()
                            .text_color(rgb(TEXT_STRONG))
                            .child(fixed(phase.stats.fps_avg, 1)),
                    ))
                    .child(table_cell(
                        div()
                            .text_color(rgb(TEXT_MUTED))
                            .child(fixed(phase.stats.fps_min, 1)),
                    ))
                    .child(table_cell(
                        div()
                            .text_color(rgb(TEXT_STRONG))
                            .child(quantity(phase.stats.frame_time_avg_ms, 2, "ms")),
                    ))
                    .child(table_cell(
                        div()
                            .text_color(rgb(TEXT_MUTED))
                            .child(quantity(phase.stats.frame_time_p95_ms, 2, "ms")),
                    ))
                    .child(table_cell(
                        div()
                            .text_color(rgb(TEXT_MUTED))
                            .child(quantity(phase.stats.frame_time_max_ms, 2, "ms")),
                    )),
            );
        }

        benchmark_block = benchmark_block.child(
            div()
                .mt(px(12.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(8.0))
                        .child(
                            stat_card()
                                .child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(rgb(TEXT_MUTED))
                                        .child("Overall Avg"),
                                )
                                .child(
                                    div()
                                        .text_size(px(14.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(TEXT_STRONG))
                                        .child(quantity(result.fps_avg, 1, "fps")),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(rgb(TEXT_MUTED))
                                        .child(quantity(result.frame_time_avg_ms, 2, "ms")),
                                ),
                        )
                        .child(
                            stat_card()
                                .child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(rgb(TEXT_MUTED))
                                        .child("Overall P95"),
                                )
                                .child(
                                    div()
                                        .text_size(px(14.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(TEXT_STRONG))
                                        .child(quantity(result.frame_time_p95_ms, 2, "ms")),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(rgb(TEXT_MUTED))
                                        .child(format!("Projection: {}", result.projection_mode)),
                                ),
                        )
                        .child(
                            stat_card()
                                .child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(rgb(TEXT_MUTED))
                                        .child("Run Duration"),
                                )
                                .child(
                                    div()
                                        .text_size(px(14.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(TEXT_STRONG))
                                        .child(quantity(result.total_duration_ms / 1000.0, 2, "s")),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(rgb(TEXT_MUTED))
                                        .child(format!(
                                            "Camera feel: {} • Stress: {}",
                                            result.camera_feel_preset, result.stress_profile
                                        )),
                                ),
                        ),
                )
                .child(phases_table),
        );
    }

    // ── sparklines ─────────────────────────────────────────────────────────
    let fps_max = stats
        .fps_history
        .iter()
        .copied()
        .fold(60.0f64, f64::max)
        .max(1.0);
    let frame_max = stats
        .frame_time_history
        .iter()
        .copied()
        .fold(24.0f64, f64::max)
        .max(1.0);
    let charts = div()
        .flex()
        .flex_wrap()
        .gap(px(16.0))
        .child(
            stat_card()
                .child(
                    div()
                        .mb(px(8.0))
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("FPS History"),
                )
                .child(sparkline(stats.fps_history.clone(), 0.0, fps_max, ACCENT))
                .child(
                    div()
                        .mt(px(4.0))
                        .text_size(px(10.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Last 120s (250ms samples)"),
                ),
        )
        .child(
            stat_card()
                .child(
                    div()
                        .mb(px(8.0))
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Frame Time History (ms)"),
                )
                .child(sparkline(
                    stats.frame_time_history.clone(),
                    0.0,
                    frame_max,
                    ACCENT_SECONDARY,
                ))
                .child(
                    div()
                        .mt(px(4.0))
                        .text_size(px(10.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Last 120s frame time"),
                ),
        );

    // ── scene / system ─────────────────────────────────────────────────────
    let mut scene_lines = div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .text_size(px(12.0));
    for (label, value) in [
        ("Mode: ", facts.app_mode.clone()),
        ("Rendering Mode: ", facts.camera_projection_mode.clone()),
        ("Graphics API: ", stats.webgl_mode.clone()),
        ("Models: ", facts.model_count.to_string().into()),
        ("Visible Models: ", facts.visible_model_count.to_string().into()),
        ("Selected Models: ", facts.selected_model_count.to_string().into()),
        ("Total Triangles: ", grouped(facts.total_polygons).into()),
        ("Selected Triangles: ", grouped(facts.selected_polygons).into()),
    ] {
        scene_lines = scene_lines.child(stat_line(label, value));
    }

    let mut system_lines = div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .text_size(px(12.0));
    for (label, value) in [
        (
            "Logical CPU Cores: ",
            facts
                .logical_cpu_cores
                .map(|cores| cores.to_string())
                .unwrap_or_else(|| "N/A".into())
                .into(),
        ),
        (
            "Device Memory: ",
            facts
                .device_memory_gb
                .map(|gb| format!("{gb} GB"))
                .unwrap_or_else(|| "N/A".into())
                .into(),
        ),
        (
            "Total JS Heap: ",
            format_bytes(stats.total_js_heap_bytes).into(),
        ),
        (
            "JS Heap Limit: ",
            format_bytes(stats.heap_limit_bytes).into(),
        ),
        ("User Agent: ", facts.user_agent.clone()),
    ] {
        system_lines = system_lines.child(stat_line(label, value));
    }

    let scene_system = div()
        .flex()
        .flex_wrap()
        .gap(px(16.0))
        .child(
            stat_card()
                .child(
                    div()
                        .mb(px(8.0))
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Scene Stats"),
                )
                .child(scene_lines),
        )
        .child(
            stat_card()
                .child(
                    div()
                        .mb(px(8.0))
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("System"),
                )
                .child(system_lines),
        );

    // ── support / picking ──────────────────────────────────────────────────
    let mut support_lines = div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .text_size(px(12.0));
    for (key, count) in &stats.support_stats.collections {
        let label: String = {
            let mut chars = key.chars();
            match chars.next() {
                Some(first) => format!("{}{}: ", first.to_uppercase(), chars.as_str()),
                None => ": ".into(),
            }
        };
        support_lines = support_lines.child(stat_line(
            label,
            grouped(*count as f64).into(),
        ));
    }
    for (label, value) in [
        (
            "Segments: ",
            grouped(stats.support_stats.segment_count as f64).into(),
        ),
        (
            "Joints: ",
            grouped(stats.support_stats.joint_count as f64).into(),
        ),
        (
            "Estimated Render Primitives: ",
            grouped(stats.support_stats.estimated_renderable_primitives as f64).into(),
        ),
    ] {
        support_lines = support_lines.child(stat_line(label, value));
    }

    let mut picking_lines = div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .text_size(px(12.0));
    for (label, value) in [
        (
            "Total Registered: ",
            grouped(stats.support_stats.picking_registered_total as f64).into(),
        ),
        (
            "Support Registered: ",
            grouped(stats.support_stats.picking_registered_support_related as f64).into(),
        ),
        (
            "Visible Pick Objects: ",
            grouped(stats.support_stats.picking_visible_objects as f64).into(),
        ),
        (
            "Cached Pick Objects: ",
            grouped(stats.support_stats.picking_cached_objects as f64).into(),
        ),
        (
            "Pick Avg: ",
            quantity(stats.support_stats.picking_avg_ms, 3, "ms").into(),
        ),
        (
            "Pick Last: ",
            quantity(stats.support_stats.picking_last_ms, 3, "ms").into(),
        ),
        (
            "Sync Cache Avg: ",
            quantity(stats.support_stats.picking_sync_avg_ms, 3, "ms").into(),
        ),
        (
            "Pick Rate: ",
            quantity(stats.support_stats.picking_picks_per_second, 1, "perSecond").into(),
        ),
    ] {
        picking_lines = picking_lines.child(stat_line(label, value));
    }

    let support_picking = div()
        .flex()
        .flex_wrap()
        .gap(px(16.0))
        .child(
            stat_card()
                .child(
                    div()
                        .mb(px(8.0))
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Support Stats"),
                )
                .child(support_lines),
        )
        .child(
            stat_card()
                .child(
                    div()
                        .mb(px(8.0))
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("GPU Picking Stats"),
                )
                .child(picking_lines),
        );

    let body = div()
        .id("diagnostics-body")
        .max_h(relative(0.88))
        .overflow_y_scroll()
        .p(px(16.0))
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(tiles)
        .child(benchmark_block)
        .child(charts)
        .child(scene_system)
        .child(support_picking);

    // Backdrop `bg-black/60`; panel `max-w-5xl` with the source's shadow.
    debug_sheet(1024.0, 0x99, 16.0, |panel| {
        panel
            .max_h(relative(0.88))
            .overflow_hidden()
            .shadow(vec![shadow(0.0, 28.0, 64.0, 0.0, with_alpha(BLACK, 0x7a))])
            .child(header)
            .child(body)
    })
}


//! Loading and status feedback: spinners, progress bars, toasts and the
//! blocking overlay.

use gpui::{
    Div, FontWeight, IntoElement, SharedString, div, linear_color_stop, linear_gradient,
    prelude::*, px, relative, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_core::state::ToastTone;
use gpui_core::theme::*;

use super::constants::*;

// ── Spinner ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SpinnerSize {
    Xs,
    Sm,
    Md,
    Lg,
    Xl,
}

/// `Spinner.tsx`: the `Loader2` glyph at `h-3.5 w-3.5` (sm).
///
/// The source turns the glyph with SMIL; GPUI has no animation, so the arc is
/// rendered statically. `currentColor` does not reach an `svg()` element in
/// GPUI, so the colour is explicit.
pub fn spinner(size: f32) -> Div {
    spinner_colored(size, TEXT_MUTED)
}

/// [`spinner`] at one of the source's size classes.
pub fn spinner_sized(size: SpinnerSize) -> Div {
    spinner(match size {
        SpinnerSize::Xs => 12.0,
        SpinnerSize::Sm => 14.0,
        SpinnerSize::Md => 16.0,
        SpinnerSize::Lg => 20.0,
        SpinnerSize::Xl => 24.0,
    })
}

/// [`spinner`] with an explicit glyph colour.
pub fn spinner_colored(size: f32, color: u32) -> Div {
    div()
        .flex_shrink_0()
        .w(px(size))
        .h(px(size))
        .child(icon("loader-circle", size, color))
}

// ── Toast ───────────────────────────────────────────────────────────────────


#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastShape {
    Pill,
    Rounded,
}

/// `Toast.tsx`: `border px-4 py-2 text-sm font-semibold shadow-lg`, `rounded-full`
/// (pill) or `rounded-lg` (rounded), tinted per tone.
///
/// GPUI has no transitions or keyframes, so the source's `animated` fade-and-rise
/// and `pulse` are not rendered; the toast is drawn in its final state.
pub fn toast(tone: ToastTone, shape: ToastShape, content: impl IntoElement) -> Div {
    let (border, background) = match tone {
        ToastTone::Neutral => (BORDER_SUBTLE, SURFACE_0),
        ToastTone::Info => (
            mix(0x60a5fa, BORDER_SUBTLE, 0.5),
            mix(0x60a5fa, SURFACE_0, 0.10),
        ),
        ToastTone::Success => (
            mix(0x22c55e, BORDER_SUBTLE, 0.5),
            mix(0x22c55e, SURFACE_0, 0.10),
        ),
        ToastTone::Warning => (
            mix(0xf59e0b, BORDER_SUBTLE, 0.5),
            mix(0xf59e0b, SURFACE_0, 0.10),
        ),
        ToastTone::Error => (
            mix(0xef4444, BORDER_SUBTLE, 0.5),
            mix(0xef4444, SURFACE_0, 0.10),
        ),
    };
    div()
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .px(px(16.0))
        .py(px(8.0))
        .text_size(px(FONT_MD))
        .line_height(px(20.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_STRONG))
        .shadow_lg()
        .rounded(px(if shape == ToastShape::Pill {
            9999.0
        } else {
            ROUNDED_LG
        }))
        .child(content)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastViewportPosition {
    BottomCenter,
    TopCenter,
}

/// `ToastViewport`: `pointer-events-none fixed inset-x-0 flex justify-center
/// px-3`, pinned to the bottom (`1.25rem`) or top (`1rem`) by the caller's
/// `offset`.
pub fn toast_viewport(
    position: ToastViewportPosition,
    offset: f32,
    content: impl IntoElement,
) -> Div {
    let mut viewport = div()
        .absolute()
        .left(px(0.0))
        .right(px(0.0))
        .flex()
        .justify_center()
        .px(px(12.0));
    viewport = match position {
        ToastViewportPosition::BottomCenter => viewport.bottom(px(offset)),
        ToastViewportPosition::TopCenter => viewport.top(px(offset)),
    };
    viewport.child(content)
}

/// [`toast_viewport`] with the source's defaults: `1.25rem` (20px) from the
/// bottom, `1rem` (16px) from the top.
pub fn toast_viewport_default(
    position: ToastViewportPosition,
    content: impl IntoElement,
) -> Div {
    let offset = match position {
        ToastViewportPosition::BottomCenter => 20.0,
        ToastViewportPosition::TopCenter => 16.0,
    };
    toast_viewport(position, offset, content)
}


// ── ProgressBar ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProgressSize {
    Xs,
    Sm,
    Md,
}

/// `ProgressBar.tsx`.
///
/// Track: `w-full overflow-hidden rounded-full` at `h-1.5` / `h-2` / `h-2.5`,
/// filled with `color-mix(surface-2, black 20%)`. Fill:
/// `linear-gradient(90deg, var(--accent), #ff79c6)`. Indeterminate renders the
/// 32% sweeping indicator statically (GPUI has no keyframes).
pub fn progress_bar(value: f32, size: ProgressSize, indeterminate: bool) -> Div {
    progress_bar_phase(value, size, indeterminate, 0.0)
}

/// `progress_bar` with an explicit indeterminate phase (0..1).
pub fn progress_bar_phase(value: f32, size: ProgressSize, indeterminate: bool, phase: f32) -> Div {
    let height = match size {
        ProgressSize::Xs => 6.0,
        ProgressSize::Sm => 8.0,
        ProgressSize::Md => 10.0,
    };
    let percent = value.clamp(0.0, 100.0);
    let fill = || {
        linear_gradient(
            90.0,
            linear_color_stop(rgb(ACCENT), 0.0),
            linear_color_stop(rgb(0xff79c6), 1.0),
        )
    };
    let mut track = div()
        .relative()
        .w_full()
        .h(px(height))
        .overflow_hidden()
        .rounded_full()
        .bg(rgb(mix(SURFACE_2, BLACK, 0.80)));
    if indeterminate {
        // The caller passes the travelling phase (0..1); the fill is 32% wide
        // and sweeps the remaining 68%.
        track = track.child(
            div()
                .absolute()
                .left(relative(phase * 0.68))
                .top(px(0.0))
                .bottom(px(0.0))
                .w(relative(0.32))
                .rounded_full()
                .bg(fill()),
        );
    } else {
        track = track.child(
            div()
                .h_full()
                .rounded_full()
                .w(relative(percent / 100.0))
                .bg(fill()),
        );
    }
    track
}


// ── BlockingOverlay ─────────────────────────────────────────────────────────

/// The built-in progress bar of [`blocking_overlay`].
pub enum BlockingProgress {
    /// `progress` omitted — no bar at all.
    Hidden,
    /// `progress === null` — the sweeping indicator.
    Indeterminate,
    /// `progress` 0–100.
    Value(f32),
}

/// `BlockingOverlay.tsx` — the non-dismissible busy sheet.
///
/// Backdrop: `absolute inset-0 flex items-center justify-center bg-black/45
/// backdrop-blur-[1px]` (the blur is omitted). Panel:
/// `w-[min(520px,92vw)] rounded-xl border px-5 py-4 shadow-xl` on
/// `color-mix(surface-0, black 10%)` with `border-subtle`.
///
/// Title `text-sm font-semibold` in `--text-strong`; details
/// `mt-1 space-y-0.5 text-xs leading-relaxed` in `--text-muted`; elapsed
/// `mt-2 text-[11px] font-medium tracking-wide` in `--accent`; footnote
/// `mt-1 text-[11px]` in `--text-muted`; the bar sits in an `mt-3` wrapper.
pub fn blocking_overlay(
    title: impl Into<SharedString>,
    details: &[SharedString],
    elapsed: Option<SharedString>,
    footnote: Option<SharedString>,
    progress: BlockingProgress,
    children: Option<impl IntoElement>,
) -> Div {
    let mut panel = div()
        // `w-[min(520px,92vw)]` — the overlay is the full window, so 92% of it
        // is 92vw.
        .w(relative(0.92))
        .max_w(px(520.0))
        .rounded(px(ROUNDED_XL))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(mix(SURFACE_0, BLACK, 0.90)))
        .px(px(20.0))
        .py(px(16.0))
        .shadow_xl()
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(px(FONT_MD))
                .line_height(px(20.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(title.into()),
        );

    if !details.is_empty() {
        let mut block = div()
            .mt(px(4.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .text_size(px(FONT_XS))
            .line_height(px(FONT_XS * LEADING_RELAXED))
            .text_color(rgb(TEXT_MUTED));
        for line in details {
            block = block.child(div().child(line.clone()));
        }
        panel = panel.child(block);
    }

    if let Some(elapsed) = elapsed {
        panel = panel.child(
            div()
                .mt(px(8.0))
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(ACCENT))
                .child(elapsed),
        );
    }

    if let Some(footnote) = footnote {
        panel = panel.child(
            div()
                .mt(px(4.0))
                .text_size(px(11.0))
                .text_color(rgb(TEXT_MUTED))
                .child(footnote),
        );
    }

    match progress {
        BlockingProgress::Hidden => {}
        BlockingProgress::Indeterminate => {
            panel = panel.child(div().mt(px(12.0)).child(progress_bar(
                0.0,
                ProgressSize::Md,
                true,
            )));
        }
        BlockingProgress::Value(value) => {
            panel = panel.child(div().mt(px(12.0)).child(progress_bar(
                value,
                ProgressSize::Md,
                false,
            )));
        }
    }

    if let Some(children) = children {
        panel = panel.child(children);
    }

    div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgba(with_alpha(BLACK, 0x73)))
        .child(panel)
}

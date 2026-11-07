//! `HistoryDebugModal.tsx` — the history-debug timeline mounted by
//! `DiagnosticsModals.tsx`.

use std::rc::Rc;

use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, SharedString, Window, div, prelude::*, px,
    relative, rgb, rgba,
};
use gpui_core::theme::*;
use gpui_ui::panel_fields::{BLACK, with_alpha};
use gpui_ui::ui::{IconButtonSize, IconButtonVariant, icon_button};

use super::chrome::{LEADING_TIGHT, debug_sheet, mix_b, shadow};
use super::format::{format_bytes_short, format_timestamp, format_history_label};

// ── HistoryDebugModal ───────────────────────────────────────────────────────

/// `HistoryDebugEvent['kind']`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HistoryDebugKind {
    Push,
    Undo,
    Redo,
    UndoEmpty,
    RedoEmpty,
    UndoHandlerMissing,
    RedoHandlerMissing,
    UndoDeclined,
    RedoDeclined,
    ClearHistory,
    ClearDebugLog,
}

impl HistoryDebugKind {
    /// `kind.replaceAll('-', ' ').replace(/\b\w/g, toUpperCase)`.
    fn label(self) -> &'static str {
        match self {
            Self::Push => "Push",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
            Self::UndoEmpty => "Undo Empty",
            Self::RedoEmpty => "Redo Empty",
            Self::UndoHandlerMissing => "Undo Handler Missing",
            Self::RedoHandlerMissing => "Redo Handler Missing",
            Self::UndoDeclined => "Undo Declined",
            Self::RedoDeclined => "Redo Declined",
            Self::ClearHistory => "Clear History",
            Self::ClearDebugLog => "Clear Debug Log",
        }
    }
}

/// One `HistoryDebugEvent`.
pub struct HistoryDebugEvent {
    pub id: u64,
    pub kind: HistoryDebugKind,
    pub timestamp: f64,
    pub payload_bytes: Option<usize>,
    pub undo_count: usize,
    pub redo_count: usize,
    pub action_type: Option<SharedString>,
    pub action_description: Option<SharedString>,
}

/// `getHistoryDebugEventVisual` — `(dot, badge background, badge border,
/// badge text, row border)`.
fn history_visual(kind: HistoryDebugKind) -> (u32, u32, u32, u32, u32) {
    match kind {
        HistoryDebugKind::Push => (
            ACCENT,
            mix_b(ACCENT, SURFACE_0, 0.90),
            mix_b(ACCENT, BORDER_SUBTLE, 0.45),
            TEXT_STRONG,
            mix_b(ACCENT, BORDER_SUBTLE, 0.68),
        ),
        HistoryDebugKind::Undo => (
            0xfbbf24,
            mix_b(0xfbbf24, SURFACE_0, 0.90),
            mix_b(0xfbbf24, BORDER_SUBTLE, 0.52),
            TEXT_STRONG,
            mix_b(0xfbbf24, BORDER_SUBTLE, 0.72),
        ),
        HistoryDebugKind::Redo => (
            0x60a5fa,
            mix_b(0x60a5fa, SURFACE_0, 0.90),
            mix_b(0x60a5fa, BORDER_SUBTLE, 0.52),
            TEXT_STRONG,
            mix_b(0x60a5fa, BORDER_SUBTLE, 0.72),
        ),
        HistoryDebugKind::UndoEmpty | HistoryDebugKind::RedoEmpty => (
            TEXT_MUTED,
            mix_b(SURFACE_2, SURFACE_0, 0.45),
            BORDER_SUBTLE,
            TEXT_MUTED,
            BORDER_SUBTLE,
        ),
        HistoryDebugKind::UndoHandlerMissing
        | HistoryDebugKind::RedoHandlerMissing
        | HistoryDebugKind::UndoDeclined
        | HistoryDebugKind::RedoDeclined => (
            0xfb7185,
            mix_b(0xfb7185, SURFACE_0, 0.90),
            mix_b(0xfb7185, BORDER_SUBTLE, 0.50),
            TEXT_STRONG,
            mix_b(0xfb7185, BORDER_SUBTLE, 0.70),
        ),
        HistoryDebugKind::ClearHistory | HistoryDebugKind::ClearDebugLog => (
            0xa78bfa,
            mix_b(0xa78bfa, SURFACE_0, 0.90),
            mix_b(0xa78bfa, BORDER_SUBTLE, 0.50),
            TEXT_STRONG,
            mix_b(0xa78bfa, BORDER_SUBTLE, 0.72),
        ),
    }
}

/// `HistoryDebugModal`'s stack and preview facts.
pub struct HistoryDebugSnapshot {
    pub events: Vec<HistoryDebugEvent>,
    pub undo_count: usize,
    pub redo_count: usize,
    /// Payload bytes held by the undo + redo stacks.
    pub payload_bytes: usize,
    /// Geometry bytes the scene-snapshot registry holds.
    pub scene_snapshot_bytes: usize,
    pub selected_preview_event_id: Option<u64>,
    pub is_preview_active: bool,
}

/// The `rounded-full border px-2 py-1 text-[11px] font-semibold` chip of the
/// history header.
fn history_chip(text: SharedString) -> Div {
    div()
        .rounded_full()
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .px(px(8.0))
        .py(px(4.0))
        .text_size(px(11.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_MUTED))
        .child(text)
}

/// A `h-8 … rounded-md border px-3 text-xs font-semibold` header button.
fn history_button(
    id: &'static str,
    label: SharedString,
    border: u32,
    background: u32,
    color: u32,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .h(px(32.0))
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .px(px(12.0))
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(color))
        .cursor_pointer()
        .on_click(on_click)
        .child(label)
}

/// `HistoryDebugModal` — the undo/redo event log.
///
/// Header `History Debug Log` / `Hotkey: Ctrl+Shift+C`, the five stack chips,
/// the four header actions, then the newest-first timeline: a dot and a
/// connector per event, the kind badge, the id and timestamp, the action label
/// and the size / undo / redo / jump tags.
#[allow(clippy::too_many_arguments)]
pub fn history_debug_modal(
    snapshot: HistoryDebugSnapshot,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_jump_to_event: impl Fn(u64, &mut Window, &mut App) + 'static,
    on_cancel_preview: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_clear_event_log: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_clear_undo_redo_stacks: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_clear_all: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
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
                        .child("History Debug Log"),
                )
                .child(
                    div()
                        .mt(px(2.0))
                        .text_size(px(11.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Hotkey: Ctrl+Shift+C"),
                ),
        )
        .child(
            icon_button(
                ElementId::named_usize("history-debug", 0),
                "x",
                IconButtonVariant::Surface,
                IconButtonSize::Sm,
            )
            .on_click(on_close),
        );

    let mut actions = div().flex().flex_wrap().gap(px(8.0));
    if snapshot.is_preview_active {
        actions = actions.child(history_button(
            "history-cancel-preview",
            "Cancel Preview".into(),
            mix_b(0xfbbf24, BORDER_SUBTLE, 0.55),
            mix_b(0xfbbf24, SURFACE_1, 0.92),
            TEXT_STRONG,
            on_cancel_preview,
        ));
    }
    actions = actions
        .child(history_button(
            "history-clear-log",
            "Clear Event Log".into(),
            BORDER_SUBTLE,
            SURFACE_1,
            TEXT_MUTED,
            on_clear_event_log,
        ))
        .child(history_button(
            "history-clear-stacks",
            "Clear Undo/Redo Stacks".into(),
            BORDER_SUBTLE,
            SURFACE_1,
            TEXT_MUTED,
            on_clear_undo_redo_stacks,
        ))
        .child(history_button(
            "history-clear-all",
            "Clear All".into(),
            mix_b(ACCENT, BORDER_SUBTLE, 0.40),
            mix_b(ACCENT, SURFACE_1, 0.92),
            TEXT_STRONG,
            on_clear_all,
        ));

    let chips = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(8.0))
        .border_b(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
        .px(px(16.0))
        .py(px(12.0))
        .child(history_chip(format!("Undo Stack: {}", snapshot.undo_count).into()))
        .child(history_chip(format!("Redo Stack: {}", snapshot.redo_count).into()))
        .child(history_chip(format!("Events: {}", snapshot.events.len()).into()))
        .child(history_chip(
            format!("Payloads: {}", format_bytes_short(snapshot.payload_bytes)).into(),
        ))
        .child(history_chip(
            format!(
                "Snapshots: {}",
                format_bytes_short(snapshot.scene_snapshot_bytes)
            )
            .into(),
        ))
        .child(div().ml_auto().child(actions));

    // Newest first.
    let on_jump_to_event = Rc::new(on_jump_to_event);
    let mut timeline = div().flex().flex_col().gap(px(6.0));
    if snapshot.events.is_empty() {
        timeline = timeline.child(
            div()
                .rounded(px(ROUNDED_LG))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_1))
                .p(px(12.0))
                .text_size(px(12.0))
                .text_color(rgb(TEXT_MUTED))
                .child("No history events yet."),
        );
    } else {
        let newest_first: Vec<&HistoryDebugEvent> = snapshot.events.iter().rev().collect();
        let last_index = newest_first.len().saturating_sub(1);
        for (index, event) in newest_first.iter().enumerate() {
            let (dot, badge_bg, badge_border, badge_text, row_border) =
                history_visual(event.kind);
            let is_last = index == last_index;
            let is_selected = snapshot.selected_preview_event_id == Some(event.id);

            let mut row = div().id(ElementId::named_usize("history-event", index)).relative().pl(px(28.0));
            if !is_last {
                // `absolute left-[10px] top-5 bottom-[-10px] w-px`.
                row = row.child(
                    div()
                        .absolute()
                        .left(px(10.0))
                        .top(px(20.0))
                        .bottom(px(-10.0))
                        .w(px(1.0))
                        .bg(rgb(mix_b(BORDER_SUBTLE, BLACK, 0.10))),
                );
            }
            row = row.child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px(10.0))
                    .w(px(20.0))
                    .h(px(20.0))
                    .rounded_full()
                    .border_1()
                    .border_color(rgb(badge_border))
                    .bg(rgb(SURFACE_0))
                    .child(
                        div()
                            .absolute()
                            .left(px(5.0))
                            .top(px(5.0))
                            .w(px(10.0))
                            .h(px(10.0))
                            .rounded_full()
                            .bg(rgb(dot)),
                    ),
            );

            let mut badges = div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(6.0))
                .text_size(px(11.0))
                .child(
                    div()
                        .rounded_full()
                        .border_1()
                        .border_color(rgb(badge_border))
                        .bg(rgb(badge_bg))
                        .px(px(8.0))
                        .py(px(2.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(badge_text))
                        .child(event.kind.label()),
                )
                .child(
                    div()
                        .text_color(rgb(TEXT_MUTED))
                        .child(format!("#{}", event.id)),
                )
                .child(
                    div()
                        .text_color(rgb(TEXT_MUTED))
                        .child(format_timestamp(event.timestamp)),
                );
            if is_selected {
                badges = badges.child(
                    div()
                        .rounded_full()
                        .border_1()
                        .border_color(rgb(mix_b(ACCENT, BORDER_SUBTLE, 0.45)))
                        .px(px(6.0))
                        .text_size(px(10.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(ACCENT))
                        .child("Preview"),
                );
            }

            let action_line: Div = if event.action_type.is_some()
                || event.action_description.is_some()
            {
                let primary: SharedString = match &event.action_description {
                    Some(description) => description.clone(),
                    None => format_history_label(
                        event.action_type.as_deref().unwrap_or(""),
                    )
                    .into(),
                };
                let mut line = div()
                    .mt(px(4.0))
                    .text_size(px(12.0))
                    .line_height(px(12.0 * LEADING_TIGHT))
                    .text_color(rgb(TEXT_STRONG))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(primary),
                    );
                if event.action_description.is_some() {
                    if let Some(action_type) = &event.action_type {
                        line = line.child(
                            div()
                                .text_color(rgb(TEXT_MUTED))
                                .child(format!(
                                    " · {}",
                                    format_history_label(action_type)
                                )),
                        );
                    }
                }
                line
            } else {
                div()
                    .mt(px(4.0))
                    .text_size(px(12.0))
                    .line_height(px(12.0 * LEADING_TIGHT))
                    .text_color(rgb(TEXT_MUTED))
                    .child("No action payload attached")
            };

            let mut tags = div()
                .mt(px(6.0))
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(6.0))
                .text_size(px(11.0))
                .text_color(rgb(TEXT_MUTED));
            let size_prefix: SharedString = match event.payload_bytes {
                Some(bytes) => format!("Size: {} · ", format_bytes_short(bytes)).into(),
                None => "".into(),
            };
            tags = tags
                .child(
                    div()
                        .rounded(px(4.0))
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .px(px(6.0))
                        .py(px(2.0))
                        .child(format!("{size_prefix}Undo: {}", event.undo_count)),
                )
                .child(
                    div()
                        .rounded(px(4.0))
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .px(px(6.0))
                        .py(px(2.0))
                        .child(format!("Redo: {}", event.redo_count)),
                )
                .child(
                    div()
                        .rounded(px(4.0))
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .px(px(6.0))
                        .py(px(2.0))
                        .child("Jump"),
                );

            let event_id = event.id;
            let jump = on_jump_to_event.clone();
            let card = div()
                .id(ElementId::named_usize("history-event-card", index))
                .cursor_pointer()
                .rounded(px(ROUNDED_LG))
                .border_1()
                .border_color(rgb(if is_selected {
                    mix_b(ACCENT, BORDER_SUBTLE, 0.40)
                } else {
                    row_border
                }))
                .bg(rgb(mix_b(SURFACE_1, BLACK, 0.08)))
                .px(px(10.0))
                .py(px(6.0))
                .on_click(move |_, window, cx| jump(event_id, window, cx))
                .child(badges)
                .child(action_line)
                .child(tags);

            row = row.child(card);
            timeline = timeline.child(row);
        }
    }

    let body = div()
        .id("history-debug-body")
        .max_h(relative(0.60))
        .overflow_y_scroll()
        .px(px(16.0))
        .py(px(12.0))
        .child(timeline);

    debug_sheet(1024.0, 0x99, 16.0, |panel| {
        panel
            .max_h(relative(0.88))
            .overflow_hidden()
            .shadow(vec![shadow(0.0, 28.0, 64.0, 0.0, with_alpha(BLACK, 0x7a))])
            .child(header)
            .child(chips)
            .child(body)
    })
}


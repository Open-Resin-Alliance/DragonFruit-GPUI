//! Scene-file, material, mesh-repair and diagnostics dialogs.
//!
//! Sources, one section per file:
//!
//! * `src/components/organisms/modals/SceneFileModals.tsx` — the unsaved-changes
//!   prompts, the scene-import placement and hollowing decisions, the plugin
//!   import warning, the save-choice sheet, the save-error sheet and the
//!   arrange blocking overlay.
//! * `src/components/modals/SceneMaterialReassignModal.tsx`
//! * `src/components/modals/ScenePrinterMismatchModal.tsx` — the
//!   printer/scene conflict dialog.
//! * `src/components/controls/ModelManagerPanel.tsx` — the plate-trash dialog.
//! * `src/components/modals/ManifoldWarningModal.tsx`
//! * `src/components/modals/TetrahedralRemeshConfirmModal.tsx`
//! * `src/components/modals/ShrinkWrapConfirmModal.tsx`
//! * `src/components/organisms/modals/MeshRepairModals.tsx` — the damaged-model
//!   dialog and the manual repair sheet.
//! * `src/components/organisms/modals/DiagnosticsModals.tsx` and the three
//!   dialogs it mounts: `DiagnosticsModal.tsx`, `HistoryDebugModal.tsx`,
//!   `SliceMetricsDebugModal.tsx`.
//!
//! Every dialog is a pure function of its data plus its callbacks; the caller
//! owns the open flag, the selection state and the async work, exactly as the
//! React components take their props. Each entry point is documented with the
//! source component it reproduces.
//!
//! # PORT DEVIATIONS
//!
//! * **z-index** — `z-[70]`, `z-[120]`, `z-[130]`, `z-[140]`, `z-[220]` have no
//!   GPUI counterpart. A dialog is painted where the caller places it, so the
//!   caller decides the stacking order.
//! * **Portals** — `StructuredDialogModal` renders through `createPortal` at
//!   the source; GPUI has no portal, so the backdrop is an `absolute` layer
//!   sized to its parent. It must be mounted in a full-size container.
//! * **`backdrop-blur-sm` / `backdrop-blur-[1px]`** — omitted; the translucent
//!   fill is kept.
//! * **`grid`** — GPUI has no grid layout. `grid-cols-2` / `grid-cols-4` /
//!   `auto-cols-fr` become wrapping flex rows whose children share the line
//!   with `flex_1`; the column count is therefore implicit in the available
//!   width instead of fixed.
//! * **`<table>`** — the material and benchmark tables are rows of divs. The
//!   source's `border-collapse` and `scope` attributes have no GPUI effect;
//!   the per-row top border and the header fill are reproduced.
//! * **`<input type="checkbox">`** — the plugin warning's "Do not remind
//!   again" box is drawn as a 14px bordered square with a `check` glyph when
//!   checked (`accentColor: #f59e0b` has no GPUI equivalent).
//! * **Transitions / animation** — `transition-colors`, the repair bar's
//!   `transition-[width] duration-300` and the spinner's SMIL rotation are
//!   static; every element is drawn in its settled state.
//! * **`title` / `aria-*`** — GPUI widgets carry no tooltip or accessible-name
//!   attribute, so the `title=` strings (full model names, byte explanations)
//!   are dropped rather than faked.
//! * **`letter-spacing`** — `tracking-wide` is not applied.
//! * **`tabular-nums`** — no font-feature control in GPUI; the digits are
//!   drawn in the regular figures of the loaded font.
//! * **`custom-scrollbar`** — the 10px accent-thumb scrollbar is not ported;
//!   the scroll containers are, with GPUI's own scrollbar.
//! * **Locale formatting** — `useLocaleFormat` formats numbers for the active
//!   Lingui locale. The port formats with `.` decimals and `,` thousands
//!   separators (the `en` catalog) and does not re-render on a locale switch.
//! * **`<svg><polyline>`** — the diagnostics sparklines are painted with
//!   `PathBuilder` on a `canvas` at the source's 2px round-capped stroke.
//! * **`conic-gradient`** — the slice-metrics pie is painted as filled wedges
//!   on a `canvas` (same colours, same percentages, same 80px ring).
//! * **`navigator` / `performance.memory`** — the diagnostics dialog's browser
//!   probes (user agent, `deviceMemory`, JS heap) are caller-supplied fields;
//!   the port does not invent them. `None` renders the source's `N/A`.
//! * **`navigator.clipboard`** — the slice-metrics "Copy" button calls the
//!   caller's `on_copy`; the port does not own a clipboard.
//! * **Escape** — `useEscapeToClose` is a window listener at the source. GPUI
//!   key handling lives in `main.rs`, so the caller decides whether Escape
//!   dismisses; the dialogs only expose their close callback.
//! * **Backdrop clicks** — the source's `onBackdropClick` is a `mousedown`
//!   handler on the backdrop. The GPUI backdrop is a plain layer with no hit
//!   handler, so a dismissal that a dialog offers only through its backdrop
//!   (the two unsaved-changes prompts, the import prompts, the repair sheet)
//!   stays with the caller; the close tile and the actions are wired.
//! * **Select trigger height** — `SceneMaterialReassignModal` passes
//!   `selectClassName="w-full h-[32px] px-2 pr-8 text-xs"`; `ui::select_dropdown`
//!   pins the trigger at the source's 36px `.ui-input` height, so the rows are
//!   4px taller than the source's.
//! * **`font-mono`** — the save-error detail block is set in the UI face; no
//!   monospace family is registered in this port.
//! * **`sticky top-0`** — the slice-metrics header scrolls with the panel; GPUI
//!   has no sticky positioning in this port.
//! * **Not ported here** — the sub-modals `SceneFileModals` mounts from other
//!   files (`ModelSupportsModal`, `ObsoleteVoxlVersionModal`,
//!   `ScenePluginMissingModal`, `SceneAutosaveRecoveryModal`,
//!   `ZipFilePickerModal`) and the ones `MeshRepairModals` mounts
//!   (`MeshRepairConfirmModal`, `MeshRepairReportModal`). They belong to their
//!   own source files and are separate ports.

#![allow(dead_code)]

use std::rc::Rc;

use gpui::{
    App, BoxShadow, ClickEvent, Div, ElementId, FontWeight, IntoElement, PathBuilder, SharedString,
    Window, canvas, div, point, prelude::*, px, relative, rgb, rgba,
};

use crate::icons::icon;
use crate::panel_fields::with_alpha;
use crate::theme::*;
use crate::ui::{
    BlockingProgress, ButtonSize, ButtonVariant, IconButtonSize, IconButtonVariant, IconTone,
    SelectOption, blocking_overlay, button, button_disabled, dialog_modal, dialog_modal_full,
    icon_button, select_dropdown, spinner,
};

/// `#ffffff`.
const WHITE: u32 = 0xffffff;
/// `#000000`.
const BLACK: u32 = 0x000000;

/// Tailwind `rounded-md`.
const ROUNDED_MD: f32 = 6.0;
/// Tailwind `rounded-lg`.
const ROUNDED_LG: f32 = 8.0;
/// Tailwind `rounded-xl`.
const ROUNDED_XL: f32 = 12.0;

/// Tailwind `leading-tight`.
const LEADING_TIGHT: f32 = 1.25;
/// Tailwind `leading-snug`.
const LEADING_SNUG: f32 = 1.375;
/// Tailwind `leading-relaxed`.
const LEADING_RELAXED: f32 = 1.625;

/// `#d97706` — amber-600, the warning accent in every dialog here.
const AMBER_600: u32 = 0xd97706;
/// `#f59e0b` — amber-500, the checkbox accent and the material note.
const AMBER_500: u32 = 0xf59e0b;
/// `#fde68a` — amber-200, the plugin warning's Continue label.
const AMBER_200: u32 = 0xfde68a;
/// `#22c55e` — green-500, the save-choice tile.
const GREEN_500: u32 = 0x22c55e;

/// `max-w-lg`.
const DIALOG_LG: f32 = 512.0;
/// `max-w-xl`.
const DIALOG_XL: f32 = 576.0;

/// A CSS `box-shadow` layer.
fn shadow(x: f32, y: f32, blur: f32, spread: f32, color: u32) -> BoxShadow {
    BoxShadow::new(px(x), px(y), rgba(color).into())
        .blur_radius(px(blur))
        .spread_radius(px(spread))
}

/// `color-mix(in srgb, a, b weight_b)` — `b`'s share is the source's percentage,
/// so `a`'s share is `1 - weight_b`.
fn mix_b(a: u32, b: u32, weight_b: f32) -> u32 {
    mix(a, b, 1.0 - weight_b)
}

// ── Shared shells ───────────────────────────────────────────────────────────

/// `StructuredDialogModal`'s backdrop: `fixed inset-0 flex items-center
/// justify-center bg-black/55 px-3`.
fn dialog_backdrop() -> Div {
    div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .px(px(12.0))
        .bg(rgba(with_alpha(BLACK, 0x8c)))
}

/// `StructuredDialogModal`'s panel: `w-full overflow-hidden rounded-xl border
/// shadow-2xl` on `surface-0`, with `0 24px 46px rgba(0,0,0,0.42)`.
fn dialog_panel(width: f32) -> Div {
    div()
        .w_full()
        .max_w(px(width))
        .rounded(px(ROUNDED_XL))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .overflow_hidden()
        .flex()
        .flex_col()
        .shadow(vec![shadow(0.0, 24.0, 46.0, 0.0, with_alpha(BLACK, 0x6b))])
}

/// The custom dialogs in `SceneFileModals` / `MeshRepairModals` repeat the
/// `StructuredDialogModal` shell by hand (`fixed inset-0 z-[220] … max-w-lg`),
/// so they share these two pieces.
fn custom_dialog(width: f32, header: Div, body: Div) -> Div {
    dialog_backdrop().child(dialog_panel(width).child(header).child(body))
}

/// The hand-written header of the custom dialogs: `flex items-center
/// justify-between gap-4 border-b px-5 py-4`, a 36px `IconChip` with the
/// caller's tint, `text-base font-semibold leading-tight` over `text-[11px]
/// leading-snug`, and a `surface` `IconButton`.
fn custom_dialog_header(
    id: &'static str,
    icon_name: &str,
    icon_border: u32,
    icon_background: u32,
    icon_foreground: u32,
    title: impl Into<SharedString>,
    subtitle: impl Into<SharedString>,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .border_b(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
        .px(px(20.0))
        .py(px(16.0))
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
                        .w(px(36.0))
                        .h(px(36.0))
                        .rounded(px(ROUNDED_MD))
                        .border_1()
                        .border_color(rgb(icon_border))
                        .bg(rgb(icon_background))
                        .child(icon(icon_name, 16.0, icon_foreground)),
                )
                .child(
                    div()
                        .min_w(px(0.0))
                        .pr(px(8.0))
                        .child(
                            div()
                                .text_size(px(16.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .line_height(px(16.0 * LEADING_TIGHT))
                                .text_color(rgb(TEXT_STRONG))
                                .child(title.into()),
                        )
                        .child(
                            div()
                                .mt(px(2.0))
                                .text_size(px(11.0))
                                .line_height(px(11.0 * LEADING_SNUG))
                                .text_color(rgb(TEXT_MUTED))
                                .child(subtitle.into()),
                        ),
                ),
        )
        .child(
            icon_button(
                ElementId::named_usize(id, 0),
                "x",
                IconButtonVariant::Surface,
                IconButtonSize::Md,
            )
            .on_click(on_close),
        )
}

/// A `text-sm leading-relaxed` paragraph in `--text-muted`.
fn muted_paragraph(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(14.0))
        .line_height(px(14.0 * LEADING_RELAXED))
        .text_color(rgb(TEXT_MUTED))
        .child(text.into())
}

/// A `text-xs leading-relaxed` paragraph in `--text-muted`.
fn muted_small_paragraph(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(12.0))
        .line_height(px(12.0 * LEADING_RELAXED))
        .text_color(rgb(TEXT_MUTED))
        .child(text.into())
}

/// One run of a paragraph: plain, or the source's `<strong>` emphasis
/// (`--text-strong`, `font-medium`).
enum Run {
    Plain(SharedString),
    Strong(SharedString),
}

impl Run {
    fn plain(text: impl Into<SharedString>) -> Self {
        Self::Plain(text.into())
    }

    fn strong(text: impl Into<SharedString>) -> Self {
        Self::Strong(text.into())
    }
}

/// A paragraph with inline emphasis. CSS flows the runs as one line box; GPUI
/// text elements are block-level, so the runs sit in a wrapping flex row and
/// wrap at run boundaries (see the module's PORT DEVIATIONS).
fn rich_paragraph(size: f32, leading: f32, runs: Vec<Run>) -> Div {
    let mut row = div()
        .flex()
        .flex_wrap()
        .items_baseline()
        .text_size(px(size))
        .line_height(px(size * leading))
        .text_color(rgb(TEXT_MUTED));
    for run in runs {
        row = row.child(match run {
            Run::Plain(text) => div().child(text),
            Run::Strong(text) => div()
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(TEXT_STRONG))
                .child(text),
        });
    }
    row
}

/// The `rounded-md border px-3 py-2` warning strip the confirm dialogs share:
/// `color-mix(#d97706, border-subtle 40%)` on `color-mix(#d97706,
/// surface-1 92%)`, with a 14px `alert-triangle` and `text-xs` copy.
fn warning_strip(text: impl Into<SharedString>) -> Div {
    div()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(mix_b(AMBER_600, BORDER_SUBTLE, 0.40)))
        .bg(rgb(mix_b(AMBER_600, SURFACE_1, 0.92)))
        .px(px(12.0))
        .py(px(8.0))
        .child(
            div()
                .flex()
                .items_start()
                .gap(px(8.0))
                .child(
                    div()
                        .mt(px(2.0))
                        .flex_shrink_0()
                        .child(icon("alert-triangle", 14.0, AMBER_600)),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .line_height(px(12.0 * LEADING_RELAXED))
                        .text_color(rgb(TEXT_MUTED))
                        .child(text.into()),
                ),
        )
}

/// The `rounded-md border px-3 py-2` identity card the import prompts and the
/// repair sheet share: `--border-subtle` on `--surface-1`, an 11px uppercase
/// label over a `text-sm font-semibold` value.
fn identity_card(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Div {
    div()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .px(px(12.0))
        .py(px(8.0))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(TEXT_MUTED))
                .child(label.into().to_uppercase()),
        )
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .truncate()
                .child(value.into()),
        )
}

/// One action of a dialog's action row. `StructuredDialogModal`'s row is
/// `grid grid-flow-col auto-cols-fr`, so every action takes an equal column.
fn action_slot(child: impl IntoElement) -> Div {
    div().flex_1().child(child)
}

// ── UnsavedChangesDialog ────────────────────────────────────────────────────

/// The shared shell of the two unsaved-changes prompts.
///
/// `UnsavedChangesDialog` in `SceneFileModals.tsx`: title `Unsaved Scene
/// Changes`, `AlertTriangle` on a warning tile, `z-[220]`, a close tile that is
/// disabled while `busy`, and the two actions `Discard Changes`
/// (`tinted-danger`, `Trash2`) and the caller's save label (`secondary`).
fn unsaved_changes_dialog(
    id: &'static str,
    subtitle: SharedString,
    body: Div,
    confirm_label: SharedString,
    busy: bool,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_discard: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_save: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let cancel = Rc::new(on_cancel);
    let close_cancel = cancel.clone();

    let mut discard = tinted_action(
        ElementId::named_usize(id, 1),
        "Discard Changes",
        Some(("trash-2", 14.0)),
        TintedTone::Danger,
    )
    .w_full();
    if busy {
        discard = discard.opacity(0.5).cursor_not_allowed();
    } else {
        discard = discard.on_click(move |event, window, cx| on_discard(event, window, cx));
    }

    let save = if busy {
        button_disabled(
            ElementId::named_usize(id, 2),
            confirm_label,
            ButtonSize::Md,
        )
        .w_full()
    } else {
        button(
            ElementId::named_usize(id, 2),
            confirm_label,
            ButtonVariant::Secondary,
            ButtonSize::Md,
        )
        .w_full()
        .on_click(move |event, window, cx| on_save(event, window, cx))
    };

    dialog_modal_full(
        DIALOG_LG,
        "Unsaved Scene Changes",
        Some(subtitle),
        Some("alert-triangle"),
        IconTone::Warning,
        busy,
        body,
        div()
            .flex()
            .gap(px(8.0))
            .child(action_slot(discard))
            .child(action_slot(save)),
        move |event, window, cx| {
            if busy {
                return;
            }
            close_cancel(event, window, cx);
        },
    )
}

/// The two tinted button tones the dialogs use with a leading glyph.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TintedTone {
    Accent,
    Danger,
}

/// A `Button` in a tinted variant with an optional 14px glyph before its label.
///
/// `Button.tsx`'s tinted classes are `.ui-button-tint` plus the tone's
/// declaration in `globals.css`; the values are the ones `ui.rs`'
/// `button_colors` carries for `TintedAccent` / `TintedDanger`. The source's
/// `className="inline-flex items-center justify-center gap-1.5"` supplies the
/// 6px gap, which `button` cannot add because it has already appended its label.
fn tinted_action(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    glyph: Option<(&str, f32)>,
    tone: TintedTone,
) -> gpui::Stateful<Div> {
    let label: SharedString = label.into();
    let (background, border, text, hover) = match tone {
        TintedTone::Accent => (
            mix(ACCENT, SURFACE_1, 0.14),
            mix(ACCENT, BORDER_SUBTLE, 0.55),
            ACCENT,
            mix(ACCENT, SURFACE_1, 0.24),
        ),
        TintedTone::Danger => (
            mix(DANGER, SURFACE_1, 0.14),
            mix(DANGER, BORDER_SUBTLE, 0.55),
            DANGER,
            mix(DANGER, SURFACE_1, 0.24),
        ),
    };
    let mut el = div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .h(px(36.0))
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .pt(px(5.0))
        .pb(px(7.0))
        .px(px(11.0))
        .text_size(px(FONT_MD))
        .font_weight(FontWeight::SEMIBOLD)
        .line_height(px(FONT_MD))
        .text_color(rgb(text))
        .cursor_pointer()
        .hover(move |style| style.bg(rgb(hover)));
    if let Some((name, size)) = glyph {
        el = el.child(icon(name, size, text));
    }
    el.child(label)
}

/// `UnsavedChangesDialog` mounted as the close-the-program prompt.
///
/// Subtitle is `You have unsaved edits in this scene.` or `This scene is
/// already saved.`; the primary action is `Save & Close`.
pub fn close_unsaved_changes_modal(
    has_unsaved_changes: bool,
    busy: bool,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_discard: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_save: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let subtitle: SharedString = if has_unsaved_changes {
        "You have unsaved edits in this scene.".into()
    } else {
        "This scene is already saved.".into()
    };
    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(muted_small_paragraph(if has_unsaved_changes {
            "You’re about to close DragonFruit with unsaved scene changes."
        } else {
            "Close DragonFruit now?"
        }))
        .child(
            div()
                .text_size(px(12.0))
                .line_height(px(12.0 * LEADING_RELAXED))
                .text_color(rgb(TEXT_MUTED))
                .font_weight(FontWeight::SEMIBOLD)
                .child("Please ensure you have saved any important work."),
        );
    unsaved_changes_dialog(
        "close-unsaved-changes",
        subtitle,
        body,
        "Save & Close".into(),
        busy,
        on_cancel,
        on_discard,
        on_save,
    )
}

/// `UnsavedChangesDialog` mounted as the new-scene prompt.
///
/// Same shell; the primary action is `Save & New Scene`.
pub fn new_scene_unsaved_changes_modal(
    has_unsaved_changes: bool,
    busy: bool,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_discard: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_save: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let subtitle: SharedString = if has_unsaved_changes {
        "You have unsaved edits in this scene.".into()
    } else {
        "This scene is already saved.".into()
    };
    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(muted_small_paragraph(if has_unsaved_changes {
            "You’re about to start a new scene with unsaved edits in this one."
        } else {
            "Start a new scene now?"
        }))
        .child(
            div()
                .text_size(px(12.0))
                .line_height(px(12.0 * LEADING_RELAXED))
                .text_color(rgb(TEXT_MUTED))
                .font_weight(FontWeight::SEMIBOLD)
                .child("Please ensure you have saved any important work."),
        );
    unsaved_changes_dialog(
        "new-scene-unsaved-changes",
        subtitle,
        body,
        "Save & New Scene".into(),
        busy,
        on_cancel,
        on_discard,
        on_save,
    )
}

// ── Scene import placement / hollowing ──────────────────────────────────────

/// A two-column button row (`grid grid-cols-2 gap-2`).
fn two_button_row(left: impl IntoElement, right: impl IntoElement) -> Div {
    div()
        .flex()
        .gap(px(8.0))
        .pt(px(4.0))
        .child(div().flex_1().child(left))
        .child(div().flex_1().child(right))
}

/// `sceneImportPlacementPrompt` — `SceneFileModals.tsx`'s inline dialog.
///
/// Title `Scene may be off-plate`, subtitle `Choose how to place imported
/// models.`, a `LayoutGrid` accent tile, the imported-file card with the
/// off-plate count, the two explanations and `Load As-Is` / `Auto-Arrange`.
/// Backdrop clicks and the close tile resolve `load_as_is`.
pub fn scene_import_placement_modal(
    file_name: SharedString,
    off_plate_model_count: usize,
    model_count: usize,
    on_load_as_is: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_auto_arrange: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let close = Rc::new(on_load_as_is);
    let header_close = close.clone();
    let header = custom_dialog_header(
        "scene-import-placement",
        "layout-grid",
        mix_b(ACCENT, BORDER_SUBTLE, 0.45),
        mix_b(ACCENT, SURFACE_1, 0.88),
        ACCENT,
        "Scene may be off-plate",
        "Choose how to place imported models.",
        move |event, window, cx| header_close(event, window, cx),
    );

    let plural = if model_count == 1 { "" } else { "s" };
    let card = div()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .px(px(12.0))
        .py(px(8.0))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(TEXT_MUTED))
                .child("IMPORTED SCENE"),
        )
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .truncate()
                .child(file_name),
        )
        .child(
            div()
                .mt(px(4.0))
                .text_size(px(12.0))
                .text_color(rgb(TEXT_MUTED))
                .child(format!(
                    "{} of {} model{plural} appear outside the build plate.",
                    grouped(off_plate_model_count as f64),
                    grouped(model_count as f64)
                )),
        );

    let explanation = div()
        .text_size(px(12.0))
        .line_height(px(12.0 * LEADING_RELAXED))
        .text_color(rgb(TEXT_MUTED))
        .child(rich_paragraph(
            12.0,
            LEADING_RELAXED,
            vec![
                Run::strong("Auto-Arrange"),
                Run::plain(" will reposition imported models onto free space on the plate."),
            ],
        ))
        .child(
            div().mt(px(4.0)).child(rich_paragraph(
                12.0,
                LEADING_RELAXED,
                vec![
                    Run::strong("Load As-Is"),
                    Run::plain(" keeps scene coordinates exactly as stored in the file."),
                ],
            )),
        );

    let load_as_is = button(
        "scene-import-load-as-is",
        "Load As-Is",
        ButtonVariant::Secondary,
        ButtonSize::Md,
    )
    .w_full()
    .on_click({
        let close = close.clone();
        move |event, window, cx| close(event, window, cx)
    });
    let auto_arrange = button(
        "scene-import-auto-arrange",
        "Auto-Arrange",
        ButtonVariant::TintedAccent,
        ButtonSize::Md,
    )
    .w_full()
    .on_click(move |event, window, cx| on_auto_arrange(event, window, cx));

    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .p(px(20.0))
        .child(card)
        .child(explanation)
        .child(two_button_row(load_as_is, auto_arrange));

    custom_dialog(DIALOG_LG, header, body)
}

/// `sceneImportHollowingPrompt` — `SceneFileModals.tsx`'s inline dialog.
///
/// Title `Scene was hollowed before export`, subtitle `Choose whether to keep
/// the hollow.`, a `CircleDashed` accent tile, the imported-file card with the
/// cavity count, the two explanations and `Import Solid` / `Keep Hollow`.
pub fn scene_import_hollowing_modal(
    file_name: SharedString,
    hollowed_model_count: usize,
    model_count: usize,
    on_import_solid: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_keep_hollow: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let close = Rc::new(on_keep_hollow);
    let header_close = close.clone();
    let header = custom_dialog_header(
        "scene-import-hollowing",
        "circle-dashed",
        mix_b(ACCENT, BORDER_SUBTLE, 0.45),
        mix_b(ACCENT, SURFACE_1, 0.88),
        ACCENT,
        "Scene was hollowed before export",
        "Choose whether to keep the hollow.",
        move |event, window, cx| header_close(event, window, cx),
    );

    let plural = if model_count == 1 { "model" } else { "models" };
    let verb = if model_count == 1 { "carries" } else { "carry" };
    let card = div()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .px(px(12.0))
        .py(px(8.0))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(TEXT_MUTED))
                .child("IMPORTED SCENE"),
        )
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .truncate()
                .child(file_name),
        )
        .child(
            div()
                .mt(px(4.0))
                .text_size(px(12.0))
                .text_color(rgb(TEXT_MUTED))
                .child(format!(
                    "{} of {model_count} {plural} {verb} an interior cavity.",
                    grouped(hollowed_model_count as f64)
                )),
        );

    let explanation = div()
        .text_size(px(12.0))
        .line_height(px(12.0 * LEADING_RELAXED))
        .text_color(rgb(TEXT_MUTED))
        .child(rich_paragraph(
            12.0,
            LEADING_RELAXED,
            vec![
                Run::strong("Keep Hollow"),
                Run::plain(
                    " builds the stored cavity into each model, so it prints hollow exactly as authored.",
                ),
            ],
        ))
        .child(
            div().mt(px(4.0)).child(rich_paragraph(
                12.0,
                LEADING_RELAXED,
                vec![
                    Run::strong("Import Solid"),
                    Run::plain(
                        " loads the outer shape only. The cavity is kept, so you can apply it later from the Hollowing panel.",
                    ),
                ],
            )),
        );

    let import_solid = button(
        "scene-import-solid",
        "Import Solid",
        ButtonVariant::Secondary,
        ButtonSize::Md,
    )
    .w_full()
    .on_click(move |event, window, cx| on_import_solid(event, window, cx));
    let keep_hollow = button(
        "scene-import-keep-hollow",
        "Keep Hollow",
        ButtonVariant::TintedAccent,
        ButtonSize::Md,
    )
    .w_full()
    .on_click({
        let close = close.clone();
        move |event, window, cx| close(event, window, cx)
    });

    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .p(px(20.0))
        .child(card)
        .child(explanation)
        .child(two_button_row(import_solid, keep_hollow));

    custom_dialog(DIALOG_LG, header, body)
}

// ── Plugin import warning ───────────────────────────────────────────────────

/// A 14px checkbox: `h-3.5 w-3.5 rounded border` with `accentColor: #f59e0b`.
fn warning_checkbox(checked: bool) -> Div {
    let mut box_ = div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .w(px(14.0))
        .h(px(14.0))
        .rounded(px(4.0))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE));
    if checked {
        box_ = box_.bg(rgb(AMBER_500)).child(icon("check", 10.0, BLACK));
    }
    box_
}

/// `showPluginImportWarningModal` — `SceneFileModals.tsx`.
///
/// Title is `{feature} is Experimental` when the caller names the feature,
/// otherwise `Plugin Import is Experimental`; subtitle `This feature is still
/// under development.`; warning tile; the body paragraph; and the footer with
/// the `Do not remind again` checkbox, `Cancel` and the amber `Continue`.
pub fn plugin_import_warning_modal(
    plugin_import_warning_title: Option<SharedString>,
    plugin_import_warning_body: Option<SharedString>,
    skip_future: bool,
    on_toggle_skip: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_continue: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let title: SharedString = match plugin_import_warning_title {
        Some(feature) => format!("{feature} is Experimental").into(),
        None => "Plugin Import is Experimental".into(),
    };
    let body_text: SharedString = plugin_import_warning_body.unwrap_or_else(|| {
        "Geometry, support placement, and transforms can import differently across scene \
         variants, so unforeseen results are still possible."
            .into()
    });

    let cancel_close = Rc::new(on_cancel);
    let close_cancel = cancel_close.clone();

    let checkbox = div()
        .id("plugin-import-skip")
        .flex()
        .items_center()
        .gap(px(8.0))
        .cursor_pointer()
        .on_click(move |event, window, cx| on_toggle_skip(event, window, cx))
        .child(warning_checkbox(skip_future))
        .child(
            div()
                .text_size(px(12.0))
                .text_color(rgb(TEXT_MUTED))
                .child("Do not remind again"),
        );

    let cancel = button(
        "plugin-import-cancel",
        "Cancel",
        ButtonVariant::Secondary,
        ButtonSize::Md,
    )
    .on_click(move |event, window, cx| close_cancel(event, window, cx));

    // `Continue` overrides the button's own colours with the amber pair:
    // `color-mix(#f59e0b, border-subtle 45%)` on `color-mix(#f59e0b,
    // surface-1 86%)`, label `#fde68a`.
    let continue_ = div()
        .id("plugin-import-continue")
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(mix_b(AMBER_500, BORDER_SUBTLE, 0.45)))
        .bg(rgb(mix_b(AMBER_500, SURFACE_1, 0.86)))
        .px(px(11.0))
        .pt(px(5.0))
        .pb(px(7.0))
        .h(px(36.0))
        .text_size(px(FONT_MD))
        .font_weight(FontWeight::SEMIBOLD)
        .line_height(px(FONT_MD))
        .text_color(rgb(AMBER_200))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(mix_b(AMBER_500, SURFACE_1, 0.76))))
        .on_click(move |event, window, cx| on_continue(event, window, cx))
        .child("Continue");

    // The source keeps the checkbox and the two buttons in the body's last row;
    // the dialog's action slot is that row here, so the shell adds no empty one.
    let footer = div()
        .flex()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .child(checkbox)
        .child(
            div()
                .flex()
                .flex_shrink_0()
                .items_center()
                .gap(px(8.0))
                .child(cancel)
                .child(continue_),
        );

    dialog_modal(
        DIALOG_LG,
        title,
        Some("This feature is still under development.".into()),
        Some("alert-triangle"),
        IconTone::Warning,
        muted_small_paragraph(body_text),
        footer,
        move |event, window, cx| cancel_close(event, window, cx),
    )
}

// ── Scene save choice / save error ──────────────────────────────────────────

/// `showSceneSaveChoiceModal` — `SceneFileModals.tsx`.
///
/// Title `Save Loaded Scene`, subtitle `Choose where Ctrl+S should save this
/// imported `.voxl` scene.`, a green `CheckCircle2` tile, the loaded-file card
/// and `Save as New Scene` / `Overwrite Loaded Scene` (disabled with no path).
pub fn scene_save_choice_modal(
    file_name: Option<SharedString>,
    path: Option<SharedString>,
    on_save_as: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_overwrite: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let close = Rc::new(on_close);
    let header_close = close.clone();
    // `color-mix(#22c55e, border-subtle 55%)` on `color-mix(#22c55e,
    // surface-1 90%)`, glyph `color-mix(#22c55e, text-strong 18%)`.
    let header = custom_dialog_header(
        "scene-save-choice",
        "circle-check",
        mix_b(GREEN_500, BORDER_SUBTLE, 0.55),
        mix_b(GREEN_500, SURFACE_1, 0.90),
        mix_b(GREEN_500, TEXT_STRONG, 0.18),
        "Save Loaded Scene",
        "Choose where Ctrl+S should save this imported `.voxl` scene.",
        move |event, window, cx| header_close(event, window, cx),
    );

    let card = div()
        .rounded(px(ROUNDED_LG))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        // `color-mix(surface-1, black 8%)`.
        .bg(rgb(mix_b(SURFACE_1, BLACK, 0.08)))
        .px(px(12.0))
        .py(px(10.0))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(TEXT_MUTED))
                .child("LOADED FILE"),
        )
        .child(
            div()
                .mt(px(4.0))
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .line_height(px(14.0 * LEADING_TIGHT))
                .text_color(rgb(TEXT_STRONG))
                .child(file_name.unwrap_or_else(|| "Loaded scene".into())),
        )
        .child(
            div()
                .mt(px(4.0))
                .text_size(px(11.0))
                .line_height(px(11.0 * LEADING_SNUG))
                .text_color(rgb(TEXT_MUTED))
                .child(path.clone().unwrap_or_else(|| {
                    "Original file path unavailable (overwrite disabled)".into()
                })),
        );

    let save_as = button(
        "scene-save-as",
        "Save as New Scene",
        ButtonVariant::Secondary,
        ButtonSize::Md,
    )
    .whitespace_nowrap()
    .on_click(move |event, window, cx| on_save_as(event, window, cx));

    let overwrite = if path.is_some() {
        button(
            "scene-save-overwrite",
            "Overwrite Loaded Scene",
            ButtonVariant::TintedAccent,
            ButtonSize::Md,
        )
        .whitespace_nowrap()
        .on_click(move |event, window, cx| on_overwrite(event, window, cx))
    } else {
        button_disabled(
            "scene-save-overwrite",
            "Overwrite Loaded Scene",
            ButtonSize::Md,
        )
        .whitespace_nowrap()
    };

    let body = div()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .p(px(20.0))
        .child(card)
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(8.0))
                .pt(px(2.0))
                .child(div().flex_1().child(save_as))
                .child(div().flex_1().child(overwrite)),
        );

    custom_dialog(DIALOG_LG, header, body)
}

/// One save failure: the title, the message and the optional detail block.
pub struct SceneSaveError {
    /// `sceneSaveError.title` — `None` renders the source's `Save error`.
    pub title: Option<SharedString>,
    pub message: SharedString,
    pub detail: Option<SharedString>,
}

/// `sceneSaveError` — `SceneFileModals.tsx`'s `StructuredDialogModal`.
///
/// Title is the error's own title (falling back to `Save error`), a 20px
/// `AlertTriangle` on the warning tile, the message in `--text-strong` and the
/// detail in a `bg-black/30` monospace block. One `OK` action.
pub fn scene_save_error_modal(
    error: SceneSaveError,
    on_dismiss: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let dismiss = Rc::new(on_dismiss);
    let close = dismiss.clone();
    let ok = dismiss.clone();

    let mut body = div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .text_size(px(12.0))
        .text_color(rgb(TEXT_MUTED))
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(TEXT_STRONG))
                .child(error.message),
        );
    if let Some(detail) = error.detail {
        body = body.child(
            div()
                .mt(px(8.0))
                .rounded(px(4.0))
                .bg(rgba(with_alpha(BLACK, 0x4d)))
                .p(px(8.0))
                .text_size(px(11.0))
                .whitespace_normal()
                .child(detail),
        );
    }

    // The source passes its own 20px amber-400 glyph; the shell's tone tile
    // paints the tone's 16px glyph (see PORT DEVIATIONS).
    dialog_modal(
        DIALOG_LG,
        error.title.unwrap_or_else(|| "Save error".into()),
        None,
        Some("alert-triangle"),
        IconTone::Warning,
        body,
        action_slot(
            button(
                "scene-save-error-ok",
                "OK",
                ButtonVariant::TintedAccent,
                ButtonSize::Md,
            )
            .on_click(move |event, window, cx| ok(event, window, cx)),
        ),
        move |event, window, cx| close(event, window, cx),
    )
}

// ── Arrange blocking overlay ────────────────────────────────────────────────

/// `showArrangeBlockingOverlay` — `BlockingOverlay` from `SceneFileModals.tsx`.
///
/// The non-dismissible arrange sheet: the translated title and detail lines,
/// `Elapsed: {elapsed}` in the accent line and `Processing N model(s)` as the
/// footnote. The progress bar is the indeterminate sweep (`progress={null}`).
pub fn arrange_blocking_overlay(
    title: SharedString,
    details: &[SharedString],
    elapsed: SharedString,
    model_count: usize,
) -> Div {
    let footnote: SharedString = if model_count == 1 {
        "Processing 1 model".into()
    } else {
        format!("Processing {} models", grouped(model_count as f64)).into()
    };
    blocking_overlay(
        title,
        details,
        Some(format!("Elapsed: {elapsed}").into()),
        Some(footnote),
        BlockingProgress::Indeterminate,
        None::<Div>,
    )
}

// ── SceneMaterialReassignModal ──────────────────────────────────────────────

/// `SceneMaterialReassignModal`'s `mode`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MaterialReassignMode {
    Reassign,
    TankMatch,
}

/// `SceneMaterialReassignOption`.
pub struct MaterialReassignOption {
    pub id: SharedString,
    pub name: SharedString,
}

/// `SceneMaterialReassignRow`.
pub struct MaterialReassignRow {
    pub id: SharedString,
    pub name: SharedString,
    pub color: Option<u32>,
}

/// `SceneMaterialTankRow`.
pub struct MaterialTankRow {
    pub id: SharedString,
    pub name: SharedString,
    pub color: Option<u32>,
    pub tank_number: u32,
}

/// The 12px material swatch: `h-3 w-3 rounded-[3px]` with an inset 1px
/// `rgba(0,0,0,0.3)` ring.
fn material_swatch(color: u32) -> Div {
    div()
        .flex_shrink_0()
        .w(px(12.0))
        .h(px(12.0))
        .rounded(px(3.0))
        .bg(rgb(color))
        .shadow(vec![shadow(0.0, 0.0, 0.0, 1.0, 0x0000004d).inset()])
}

/// A material name with its swatch, the `inline-flex items-center gap-1.5`
/// cell content of both tables.
fn material_cell(row_name: SharedString, color: Option<u32>) -> Div {
    let mut cell = div().flex().items_center().gap(px(6.0));
    if let Some(color) = color {
        cell = cell.child(material_swatch(color));
    }
    cell.child(row_name)
}

/// One table header cell: `p-2 font-medium`.
fn table_header_cell(label: impl Into<SharedString>) -> Div {
    div()
        .flex_1()
        .min_w(px(0.0))
        .p(px(8.0))
        .font_weight(FontWeight::MEDIUM)
        .child(label.into())
}

/// One table body cell: `p-2`.
fn table_cell(child: impl IntoElement) -> Div {
    div()
        .flex_1()
        .min_w(px(0.0))
        .p(px(8.0))
        .child(child)
}

/// The table frame: `overflow-hidden rounded-md border` on `--border-subtle`.
fn table_frame() -> Div {
    div()
        .overflow_hidden()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
}

/// The table header row: `background: var(--surface-1)`, `--text-muted`.
fn table_head() -> Div {
    div()
        .flex()
        .bg(rgb(SURFACE_1))
        .text_color(rgb(TEXT_MUTED))
}

/// A table body row: `border-t` on `--border-subtle`.
fn table_row() -> Div {
    div()
        .flex()
        .items_start()
        .border_t(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
}

/// `SceneMaterialReassignModal` — shown when the selected printer changes and
/// the scene already names materials from the old one.
///
/// The three table shapes are the source's: the tank-match table
/// (`{previous} | {new}` with `Tank N`), the reassign table (scene material →
/// a `SelectDropdown` per row) and the no-match table (`N/A` per row plus the
/// amber hint). `mapping` is the caller's `Record<rowId, optionId>`; `open_row`
/// is the index of the row whose menu is open.
#[allow(clippy::too_many_arguments)]
pub fn scene_material_reassign_modal(
    previous_printer_name: SharedString,
    new_printer_name: SharedString,
    mode: MaterialReassignMode,
    can_reassign: bool,
    rows: &[MaterialReassignRow],
    options: &[MaterialReassignOption],
    tank_rows: &[MaterialTankRow],
    can_assign_tanks: bool,
    mapping: &[(SharedString, SharedString)],
    open_row: Option<usize>,
    on_keep_previous: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_unassign: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_apply: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_assign_tanks: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_toggle_row: impl Fn(usize, &mut Window, &mut App) + 'static,
    on_select_row: impl Fn(usize, usize, &mut Window, &mut App) + 'static,
) -> Div {
    let is_tank_match = mode == MaterialReassignMode::TankMatch;
    let title: SharedString = if is_tank_match {
        "Assign scene materials to tanks".into()
    } else {
        "Reassign scene materials".into()
    };
    let subtitle: SharedString = if is_tank_match {
        "The new printer shares these materials but its tanks are not assigned".into()
    } else if can_reassign {
        "The new printer has different material profiles".into()
    } else {
        "The new printer has no matching materials".into()
    };

    let keep_previous = Rc::new(on_keep_previous);
    let close_keep = keep_previous.clone();
    let action_keep = keep_previous.clone();
    let unassign = Rc::new(on_unassign);

    // ── intro copy ─────────────────────────────────────────────────────────
    let mut intro = div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .text_size(px(14.0))
        .line_height(px(14.0 * LEADING_RELAXED))
        .text_color(rgb(TEXT_MUTED));
    if is_tank_match {
        intro = intro
            .child(rich_paragraph(
                14.0,
                LEADING_RELAXED,
                vec![
                    Run::plain("This scene names materials from "),
                    Run::strong(previous_printer_name.clone()),
                    Run::plain("."),
                ],
            ))
            .child(rich_paragraph(
                14.0,
                LEADING_RELAXED,
                vec![
                    Run::strong(new_printer_name.clone()),
                    Run::plain(
                        " shares them, but its tanks are not assigned. Assign each material to \
                         the matching tank, or keep the previous printer.",
                    ),
                ],
            ));
    } else {
        intro = intro.child(rich_paragraph(
            14.0,
            LEADING_RELAXED,
            vec![
                Run::plain("This scene names materials from "),
                Run::strong(previous_printer_name.clone()),
                Run::plain("."),
            ],
        ));
        let second: Vec<Run> = if can_reassign {
            vec![
                Run::plain("Choose which materials on "),
                Run::strong(new_printer_name.clone()),
                Run::plain(" the scene should use."),
            ]
        } else if rows.len() == 1 {
            vec![
                Run::plain("This material is not available on "),
                Run::strong(new_printer_name.clone()),
                Run::plain("."),
            ]
        } else {
            vec![
                Run::plain("None of these materials are available on "),
                Run::strong(new_printer_name.clone()),
                Run::plain("."),
            ]
        };
        intro = intro.child(rich_paragraph(14.0, LEADING_RELAXED, second));
    }

    // ── tables ─────────────────────────────────────────────────────────────
    let table: Div = if is_tank_match {
        let mut head = table_head();
        head = head
            .child(table_header_cell(previous_printer_name.clone()))
            .child(table_header_cell(new_printer_name.clone()));
        let mut body = div().flex().flex_col();
        for (index, row) in tank_rows.iter().enumerate() {
            let mut line = table_row().id(ElementId::named_usize("material-tank-row", index));
            line = line
                .child(table_cell(material_cell(
                    row.name.clone(),
                    row.color,
                )))
                .child(table_cell(format!("Tank {}", row.tank_number)));
            body = body.child(line);
        }
        table_frame().child(head).child(body)
    } else if can_reassign {
        let mut head = table_head();
        head = head
            .child(table_header_cell("Scene material"))
            .child(table_header_cell("Use on this printer"));
        let select_options: Vec<SelectOption> = options
            .iter()
            .map(|option| SelectOption::new(option.id.clone(), option.name.clone()))
            .collect();
        let on_toggle_row = Rc::new(on_toggle_row);
        let on_select_row = Rc::new(on_select_row);
        let mut body = div().flex().flex_col();
        for (index, row) in rows.iter().enumerate() {
            let selected = mapping
                .iter()
                .find(|(id, _)| id == &row.id)
                .map(|(_, value)| value.clone())
                .unwrap_or_else(|| {
                    options
                        .first()
                        .map(|option| option.id.clone())
                        .unwrap_or_default()
                });
            let open = open_row == Some(index);
            let toggle_row = on_toggle_row.clone();
            let toggle = move |_event: &ClickEvent, window: &mut Window, cx: &mut App| {
                toggle_row(index, window, cx)
            };
            let select_row = on_select_row.clone();
            let select = move |option_index: usize, window: &mut Window, cx: &mut App| {
                select_row(index, option_index, window, cx)
            };
            let dropdown = select_dropdown(
                format!("material-reassign-select-{index}"),
                &select_options,
                &selected,
                open,
                false,
                toggle,
                select,
            )
            .w_full();
            let line = table_row()
                .id(ElementId::named_usize("material-reassign-row", index))
                .child(table_cell(material_cell(row.name.clone(), row.color)))
                .child(table_cell(dropdown));
            body = body.child(line);
        }
        table_frame().child(head).child(body)
    } else {
        let mut head = table_head();
        head = head
            .child(table_header_cell(previous_printer_name.clone()))
            .child(table_header_cell(new_printer_name.clone()));
        let mut body = div().flex().flex_col();
        for (index, row) in rows.iter().enumerate() {
            let line = table_row()
                .id(ElementId::named_usize("material-nomatch-row", index))
                .child(table_cell(material_cell(row.name.clone(), row.color)))
                .child(
                    table_cell(
                        div()
                            .text_color(rgb(TEXT_MUTED))
                            .child("N/A"),
                    ),
                );
            body = body.child(line);
        }
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(table_frame().child(head).child(body))
            .child(muted_paragraph(
                "The new printer has no matching materials assigned to its tanks. Load without \
                 materials, or keep the previous printer to preserve them.",
            ))
            .child(
                div()
                    .text_size(px(12.0))
                    .line_height(px(12.0 * LEADING_RELAXED))
                    .text_color(rgb(AMBER_500))
                    .child(
                        "You can share materials with printers that use the same material \
                         settings and output format in Material Settings.",
                    ),
            )
    };

    // ── actions ────────────────────────────────────────────────────────────
    let mut actions = div().flex().gap(px(8.0));
    actions = actions.child(action_slot(
        button(
            "material-reassign-keep",
            format!("Keep {previous_printer_name}"),
            ButtonVariant::Secondary,
            ButtonSize::Md,
        )
        .on_click(move |event, window, cx| action_keep(event, window, cx)),
    ));
    if !can_reassign && !(is_tank_match && can_assign_tanks) {
        let unassign = unassign.clone();
        actions = actions.child(action_slot(
            button(
                "material-reassign-unassign",
                "Load without materials",
                ButtonVariant::Secondary,
                ButtonSize::Md,
            )
            .on_click(move |event, window, cx| unassign(event, window, cx)),
        ));
    }
    if can_reassign {
        actions = actions.child(action_slot(
            button(
                "material-reassign-apply",
                "Reassign materials",
                ButtonVariant::TintedAccent,
                ButtonSize::Md,
            )
            .on_click(move |event, window, cx| on_apply(event, window, cx)),
        ));
    }
    if is_tank_match && can_assign_tanks {
        actions = actions.child(action_slot(
            button(
                "material-reassign-tanks",
                "Assign to matching tanks",
                ButtonVariant::TintedAccent,
                ButtonSize::Md,
            )
            .on_click(move |event, window, cx| on_assign_tanks(event, window, cx)),
        ));
    }

    dialog_modal(
        DIALOG_XL,
        title,
        Some(subtitle),
        Some("palette"),
        IconTone::Warning,
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(intro)
            .child(table),
        actions,
        move |event, window, cx| close_keep(event, window, cx),
    )
}

// ── ScenePrinterMismatchModal ───────────────────────────────────────────────

/// `BuildVolumeMm`.
#[derive(Clone, Copy)]
pub struct BuildVolumeMm {
    pub width: f64,
    pub depth: f64,
    pub height: f64,
}

impl BuildVolumeMm {
    fn matches(&self, other: &Self) -> bool {
        let same = |a: f64, b: f64| (a - b).abs() < 1e-6;
        same(self.width, other.width)
            && same(self.depth, other.depth)
            && same(self.height, other.height)
    }

    fn label(&self) -> String {
        format!(
            "{} × {} × {} mm",
            num(self.width),
            num(self.depth),
            num(self.height)
        )
    }
}

/// `ScenePrinterMismatchModal`'s props.
pub struct ScenePrinterMismatch {
    pub recorded_printer_name: Option<SharedString>,
    pub recorded_build_volume: BuildVolumeMm,
    pub current_printer_name: SharedString,
    pub current_build_volume: BuildVolumeMm,
    pub will_add_printer: bool,
    pub current_is_smaller: bool,
    pub current_has_required_materials: bool,
    pub scene_uses_multi_tank_materials: bool,
    pub current_material_mode: Option<SharedString>,
    pub missing_plugin_name: Option<SharedString>,
}

/// `ScenePrinterMismatchModal` — the printer/scene conflict dialog.
///
/// Three variants, chosen exactly as the source chooses them: the missing
/// plugin (`Printer plugin not installed`), the lost material assignments
/// (`Built for Multi-Material`), and the plain mismatch (`Built for another
/// printer`). `Manage Plugins` is only offered in the missing-plugin variant.
#[allow(clippy::too_many_arguments)]
pub fn scene_printer_mismatch_modal(
    mismatch: ScenePrinterMismatch,
    on_switch: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_keep: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_manage_plugins: Option<impl Fn(&ClickEvent, &mut Window, &mut App) + 'static>,
) -> Div {
    let recorded_label: SharedString = mismatch
        .recorded_printer_name
        .as_ref()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "the printer this scene was built for".into())
        .into();
    let plugin_missing = mismatch
        .missing_plugin_name
        .as_ref()
        .is_some_and(|name| !name.trim().is_empty());
    let loses_material_assignments = mismatch.scene_uses_multi_tank_materials
        && mismatch
            .current_material_mode
            .as_deref()
            .is_none_or(|mode| mode != "multiTank");
    let lacks_required_materials = !plugin_missing && !mismatch.current_has_required_materials;
    let build_volumes_match = mismatch
        .recorded_build_volume
        .matches(&mismatch.current_build_volume);

    let title: SharedString = if plugin_missing {
        "Printer plugin not installed".into()
    } else if loses_material_assignments {
        "Built for Multi-Material".into()
    } else {
        "Built for another printer".into()
    };
    let subtitle: SharedString = if plugin_missing {
        "Only the geometry can be imported".into()
    } else if lacks_required_materials {
        "The selected printer is missing the scene's materials".into()
    } else if loses_material_assignments {
        "This scene is made for a multi-material printer".into()
    } else if mismatch.current_is_smaller {
        "This scene may not fit the one selected".into()
    } else {
        "Its plates are spaced for that printer".into()
    };

    let keep_close = Rc::new(on_keep);
    let close_keep = keep_close.clone();
    let action_keep = keep_close.clone();

    // ── body ───────────────────────────────────────────────────────────────
    let mut body = div().flex().flex_col().gap(px(16.0));
    if plugin_missing {
        let plugin_name = mismatch.missing_plugin_name.clone().unwrap_or_default();
        body = body
            .child(rich_paragraph(
                14.0,
                LEADING_RELAXED,
                vec![
                    Run::plain("This scene was saved for "),
                    Run::strong(recorded_label.clone()),
                    Run::plain(", whose printer and material profiles come from the "),
                    Run::strong(plugin_name),
                    Run::plain(
                        " plugin. That plugin is not installed, so those profiles cannot be \
                         restored.",
                    ),
                ],
            ))
            .child(muted_paragraph(
                "Only the geometry will be imported. Install the plugin, then reopen the scene, \
                 to restore its printer and materials.",
            ));
    } else {
        if lacks_required_materials {
            body = body.child(rich_paragraph(
                14.0,
                LEADING_RELAXED,
                vec![
                    Run::strong(mismatch.current_printer_name.clone()),
                    Run::plain(" does not have the materials this scene uses. Load the geometry "),
                    Run::plain("without them, or switch to "),
                    Run::strong(recorded_label.clone()),
                    Run::plain(" to restore them."),
                ],
            ));
        }
        if loses_material_assignments {
            body = body.child(muted_paragraph(
                "Its materials are split across several tanks, which the printer selected now \
                 cannot print. The assignments stay with the scene, hidden while this printer is \
                 selected — switch to a multi-material printer to print them.",
            ));
        }
        if !build_volumes_match {
            let recorded_volume = mismatch.recorded_build_volume.label();
            let current_volume = mismatch.current_build_volume.label();
            let mut lines = div().flex().flex_col().gap(px(16.0));
            lines = lines
                .child(rich_paragraph(
                    14.0,
                    LEADING_RELAXED,
                    vec![
                        Run::strong(recorded_label.clone()),
                        Run::plain(" has a build volume of "),
                        Run::plain(recorded_volume),
                        Run::plain("."),
                    ],
                ))
                .child(rich_paragraph(
                    14.0,
                    LEADING_RELAXED,
                    if mismatch.current_is_smaller {
                        vec![
                            Run::plain("The printer selected now, "),
                            Run::strong(mismatch.current_printer_name.clone()),
                            Run::plain(", has a build volume of "),
                            Run::plain(current_volume),
                            Run::plain(". A plate packed for the larger machine will not fit."),
                        ]
                    } else {
                        vec![
                            Run::plain("The printer selected now, "),
                            Run::strong(mismatch.current_printer_name.clone()),
                            Run::plain(", has a build volume of "),
                            Run::plain(current_volume),
                            Run::plain(", and this scene was laid out for the other one."),
                        ]
                    },
                ))
                .child(rich_paragraph(
                    14.0,
                    LEADING_RELAXED,
                    if mismatch.will_add_printer {
                        vec![
                            Run::plain("It is not in your printer library, so switching adds "),
                            Run::strong(recorded_label.clone()),
                            Run::plain(" from this scene."),
                        ]
                    } else {
                        vec![
                            Run::plain("Switch to "),
                            Run::strong(recorded_label.clone()),
                            Run::plain(" before importing?"),
                        ]
                    },
                ));
            body = body.child(lines);
        }
    }

    // ── actions ────────────────────────────────────────────────────────────
    let mut actions = div().flex().gap(px(8.0));
    if plugin_missing {
        if let Some(on_manage_plugins) = on_manage_plugins {
            actions = actions.child(action_slot(
                button(
                    "printer-mismatch-manage-plugins",
                    "Manage Plugins",
                    ButtonVariant::Secondary,
                    ButtonSize::Md,
                )
                .on_click(on_manage_plugins),
            ));
        }
        actions = actions.child(action_slot(
            button(
                "printer-mismatch-import-geometry",
                "Import Geometry",
                ButtonVariant::TintedAccent,
                ButtonSize::Md,
            )
            .on_click(move |event, window, cx| action_keep(event, window, cx)),
        ));
    } else if lacks_required_materials {
        actions = actions
            .child(action_slot(
                button(
                    "printer-mismatch-load-without-materials",
                    "Load without materials",
                    ButtonVariant::Secondary,
                    ButtonSize::Md,
                )
                .on_click(move |event, window, cx| action_keep(event, window, cx)),
            ))
            .child(action_slot(
                button(
                    "printer-mismatch-switch",
                    format!("Switch to {recorded_label}"),
                    ButtonVariant::TintedAccent,
                    ButtonSize::Md,
                )
                .on_click(move |event, window, cx| on_switch(event, window, cx)),
            ));
    } else {
        actions = actions
            .child(action_slot(
                button(
                    "printer-mismatch-keep",
                    format!("Keep {}", mismatch.current_printer_name),
                    ButtonVariant::Secondary,
                    ButtonSize::Md,
                )
                .on_click(move |event, window, cx| action_keep(event, window, cx)),
            ))
            .child(action_slot(
                button(
                    "printer-mismatch-switch",
                    format!("Switch to {recorded_label}"),
                    ButtonVariant::TintedAccent,
                    ButtonSize::Md,
                )
                .on_click(move |event, window, cx| on_switch(event, window, cx)),
            ));
    }

    dialog_modal(
        DIALOG_LG,
        title,
        Some(subtitle),
        Some("alert-triangle"),
        IconTone::Warning,
        body,
        actions,
        move |event, window, cx| close_keep(event, window, cx),
    )
}

// ── Plate trash ─────────────────────────────────────────────────────────────

/// `platePendingDelete` — `ModelManagerPanel.tsx`'s `StructuredDialogModal`.
///
/// Title `Delete this plate?`, the plate's name as the subtitle, a `Trash2`
/// danger tile, the delete copy and `Cancel` / `Delete plate`.
pub fn plate_delete_modal(
    plate_name: SharedString,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_delete: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let cancel = Rc::new(on_cancel);
    let close = cancel.clone();
    let action_cancel = cancel.clone();

    dialog_modal(
        DIALOG_LG,
        "Delete this plate?",
        Some(plate_name),
        Some("trash-2"),
        IconTone::Danger,
        muted_paragraph(
            "The models on this plate are deleted with it. Undo brings them back.",
        ),
        div()
            .flex()
            .gap(px(8.0))
            .child(action_slot(
                button(
                    "plate-delete-cancel",
                    "Cancel",
                    ButtonVariant::Secondary,
                    ButtonSize::Md,
                )
                .on_click(move |event, window, cx| action_cancel(event, window, cx)),
            ))
            .child(action_slot(
                button(
                    "plate-delete-confirm",
                    "Delete plate",
                    ButtonVariant::TintedDanger,
                    ButtonSize::Md,
                )
                .on_click(move |event, window, cx| on_delete(event, window, cx)),
            )),
        move |event, window, cx| close(event, window, cx),
    )
}

// ── Manifold / remesh / shrink-wrap ─────────────────────────────────────────

/// `ManifoldWarningModal` — the mesh-issue warning shown when validation fails.
///
/// Title `Mesh Issue detected`, subtitle `This mesh isn't a valid solid`,
/// warning tile, the three explanations, `Dismiss` and `Repair Now` (`Wrench`).
/// The source passes no `onClose` and a no-op `onBackdropClick`, so the close
/// tile is painted but inert; the user leaves through `Dismiss` or `Repair Now`.
pub fn manifold_warning_modal(
    on_dismiss: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_repair_now: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let dismiss = Rc::new(on_dismiss);
    let action_dismiss = dismiss.clone();

    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(muted_paragraph(
            "Parts of this mesh are not closed — edges are left open, faces cross through each \
             other, or surfaces do not quite meet. Slicing and support placement expect a sealed \
             solid, so defects can reach the print without showing up here first.",
        ))
        .child(muted_paragraph(
            "The red-striped areas in the viewport mark the parts that failed validation.",
        ))
        .child(muted_paragraph(
            "You can dismiss this and keep working, or repair the mesh now.",
        ));

    dialog_modal(
        DIALOG_LG,
        "Mesh Issue detected",
        Some("This mesh isn't a valid solid".into()),
        Some("alert-triangle"),
        IconTone::Warning,
        body,
        div()
            .flex()
            .gap(px(8.0))
            .child(action_slot(
                button(
                    "manifold-dismiss",
                    "Dismiss",
                    ButtonVariant::Secondary,
                    ButtonSize::Md,
                )
                .on_click(move |event, window, cx| action_dismiss(event, window, cx)),
            ))
            .child(action_slot(
                tinted_action(
                    "manifold-repair-now",
                    "Repair Now",
                    Some(("wrench", 14.0)),
                    TintedTone::Accent,
                )
                .on_click(move |event, window, cx| on_repair_now(event, window, cx)),
            )),
        |_, _, _| {},
    )
}

/// `TetrahedralRemeshConfirmModal` — confirms a remesh before it runs.
///
/// Title `Remesh this model?`, subtitle `Rebuilds the surface`, an accent
/// `Pyramid` tile, the warning strip and `Cancel` / `Remesh`. The source has no
/// `onClose` and a no-op backdrop click, so the close tile is inert.
pub fn tetrahedral_remesh_confirm_modal(
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_confirm: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let cancel = Rc::new(on_cancel);
    let action_cancel = cancel.clone();

    dialog_modal(
        DIALOG_LG,
        "Remesh this model?",
        Some("Rebuilds the surface".into()),
        Some("pyramid"),
        IconTone::Accent,
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(muted_paragraph(
                "This rebuilds the model's surface, keeping fine detail.",
            ))
            .child(warning_strip(
                "The rebuilt surface will replace this model in your current scene. Large models \
                 take longer, and some models cannot be rebuilt at all and are left untouched. \
                 You can undo this afterwards.",
            )),
        div()
            .flex()
            .gap(px(8.0))
            .child(action_slot(
                button(
                    "remesh-cancel",
                    "Cancel",
                    ButtonVariant::Secondary,
                    ButtonSize::Md,
                )
                .on_click(move |event, window, cx| action_cancel(event, window, cx)),
            ))
            .child(action_slot(
                tinted_action(
                    "remesh-confirm",
                    "Remesh",
                    Some(("pyramid", 14.0)),
                    TintedTone::Accent,
                )
                .on_click(move |event, window, cx| on_confirm(event, window, cx)),
            )),
        |_, _, _| {},
    )
}

/// `ShrinkWrapConfirmModal` — confirms a shrink-wrap before it runs.
///
/// Title `Shrink-Wrap this model?`, subtitle `Reworks the surface into one
/// solid`, an accent `RefreshCw` tile, the warning strip and `Cancel` /
/// `Shrink-Wrap`. The source has no `onClose` and a no-op backdrop click, so the
/// close tile is inert.
pub fn shrink_wrap_confirm_modal(
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_confirm: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let cancel = Rc::new(on_cancel);
    let action_cancel = cancel.clone();

    dialog_modal(
        DIALOG_LG,
        "Shrink-Wrap this model?",
        Some("Reworks the surface into one solid".into()),
        Some("refresh-cw"),
        IconTone::Accent,
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(muted_paragraph(
                "This merges overlapping shells, closes gaps and drops loose pieces, leaving one \
                 solid.",
            ))
            .child(warning_strip(
                "The rebuilt surface will replace this model in your current scene. A model that \
                 cannot be resolved is left untouched, and you can undo this afterwards.",
            )),
        div()
            .flex()
            .gap(px(8.0))
            .child(action_slot(
                button(
                    "shrink-wrap-cancel",
                    "Cancel",
                    ButtonVariant::Secondary,
                    ButtonSize::Md,
                )
                .on_click(move |event, window, cx| action_cancel(event, window, cx)),
            ))
            .child(action_slot(
                tinted_action(
                    "shrink-wrap-confirm",
                    "Shrink-Wrap",
                    Some(("refresh-cw", 14.0)),
                    TintedTone::Accent,
                )
                .on_click(move |event, window, cx| on_confirm(event, window, cx)),
            )),
        |_, _, _| {},
    )
}

// ── MeshRepairModals ────────────────────────────────────────────────────────

/// `StructuredDialog_damagedModel` — `MeshRepairModals.tsx`.
///
/// Title `Mesh quality too low`, subtitle `Boolean operation requires a
/// manifold mesh`, a danger `AlertTriangle`, `closeDisabled`, and one `Got it`
/// action. The close tile is painted disabled and carries no handler.
pub fn damaged_model_modal(
    on_dismiss: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let dismiss = Rc::new(on_dismiss);
    let action_dismiss = dismiss.clone();

    dialog_modal_full(
        DIALOG_LG,
        "Mesh quality too low",
        Some("Boolean operation requires a manifold mesh".into()),
        Some("alert-triangle"),
        IconTone::Danger,
        true,
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(muted_small_paragraph(
                "This mesh contains too many self-intersecting triangles and non-manifold edges \
                 for boolean operations to succeed. The import repair pass reduced but could not \
                 fully resolve all issues in the geometry.",
            ))
            .child(rich_paragraph(
                12.0,
                LEADING_RELAXED,
                vec![
                    Run::plain(
                        "We recommend repairing the mesh in a dedicated 3D modeling program such \
                         as ",
                    ),
                    Run::strong("Blender"),
                    Run::plain(" or "),
                    Run::strong("Netfabb"),
                    Run::plain(" before importing it into DragonFruit."),
                ],
            )),
        action_slot(
            button(
                "damaged-model-ok",
                "Got it",
                ButtonVariant::TintedAccent,
                ButtonSize::Md,
            )
            .on_click(move |event, window, cx| action_dismiss(event, window, cx)),
        ),
        move |_event, _window, _cx| {},
    )
}

/// The live state of a manual repair run.
pub struct RepairProgressState {
    pub model_name: Option<SharedString>,
    pub model_index: usize,
    pub model_count: usize,
    pub stage: Option<SharedString>,
    /// `state.stages.length` — the number of progress events seen so far.
    pub stage_events: usize,
}

/// `translateRepairStage` from
/// `src/components/scene/repairProgressMessages.ts`.
///
/// The engine reports a language-neutral stage code; only the label is
/// translated, and an unknown code falls back to the raw code rather than
/// rendering blank.
fn translate_repair_stage(stage: Option<&str>) -> String {
    let Some(stage) = stage else {
        return "Starting".into();
    };
    match stage {
        "repair" => "Repairing",
        "rebuild" => "Rebuilding the surface into a solid",
        "orient_components" => "Pointing faces the same way",
        "stitch" => "Joining loose edges",
        "holes_trivial" => "Closing small holes",
        "double_faces" => "Removing doubled faces",
        "orient_holes_stitch" => "Closing gaps",
        "orient_holes_all_stitch" => "Closing the remaining gaps",
        "double_faces_after_split" => "Removing doubled faces again",
        "split_self_intersections" => "Separating surfaces that overlap",
        "split_non_manifold_edges" => "Separating surfaces that share an edge",
        "union_overlapping_parts" => "Merging overlapping parts",
        "filter_components" => "Keeping the largest pieces",
        "remove_inner_faces" => "Removing faces inside the model",
        "noise_shells" => "Discarding scraps",
        "settle" => "Tidying up",
        "prune_and_classify" => "Separating model and supports",
        other => other,
    }
    .to_string()
}

/// `repairProgressFraction` — how full the bar is, 0..1.
///
/// The stage count is unknown in advance, so the fraction is a saturating
/// function of the events seen: it always moves and never claims the work is
/// nearly done.
fn repair_progress_fraction(state: &RepairProgressState) -> f32 {
    let model_fraction = if state.stage.is_some() {
        0.1 + 0.8 * (1.0 - (-(state.stage_events as f32) / 6.0).exp())
    } else {
        0.06
    };
    (state.model_index as f32 + model_fraction) / (state.model_count.max(1) as f32)
}

/// `formatElapsed` — `0:04.2`.
fn format_elapsed(ms: f64) -> String {
    let total_seconds = ms.max(0.0) / 1000.0;
    let minutes = (total_seconds / 60.0).floor();
    let seconds = total_seconds - minutes * 60.0;
    format!("{}:{:04.1}", minutes as u64, seconds)
}

/// `RepairProgressPanel` — the model position, the stage label, the elapsed
/// clock and the amber bar.
fn repair_progress_panel(
    state: &RepairProgressState,
    elapsed_ms: f64,
    cancelling: bool,
) -> Div {
    let stage_label: SharedString = if cancelling {
        "Stopping at the end of this step…".into()
    } else {
        translate_repair_stage(state.stage.as_deref()).into()
    };

    let mut panel = div().flex().flex_col().gap(px(12.0));
    if state.model_count > 1 {
        if let Some(name) = &state.model_name {
            panel = panel.child(
                div()
                    .mx(px(6.0))
                    .truncate()
                    .text_size(px(11.0))
                    .text_color(rgb(TEXT_MUTED))
                    .child(format!(
                        "Model {} of {}: {name}",
                        state.model_index + 1,
                        state.model_count
                    )),
            );
        }
    }
    panel = panel
        .child(
            div()
                .mx(px(6.0))
                .flex()
                .items_baseline()
                .justify_between()
                .gap(px(12.0))
                .child(
                    div()
                        .min_w(px(0.0))
                        .truncate()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(TEXT_STRONG))
                        .child(stage_label),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_size(px(11.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child(format_elapsed(elapsed_ms)),
                ),
        )
        .child(
            div()
                .mx(px(6.0))
                .h(px(6.0))
                .overflow_hidden()
                .rounded_full()
                .bg(rgb(SURFACE_2))
                .child(
                    div()
                        .h_full()
                        .rounded_full()
                        .w(relative(repair_progress_fraction(state)))
                        .bg(rgb(AMBER_600)),
                ),
        );
    panel
}

/// The manual repair sheet — `MeshRepairModals.tsx`'s `manualRepairModelIds`
/// dialog.
///
/// Title is `Repairing Mesh` when the run auto-started, otherwise
/// `Repair Mesh` / `Repair N Meshes`; subtitle `Fixing geometry issues.` or
/// `Fixes geometry issues before you keep working.`; the model card, the
/// progress panel (or the one-line explanation), the amber warning strip and
/// the `Keep Original` / `Stop` / `Repair` footer.
#[allow(clippy::too_many_arguments)]
pub fn repair_mesh_modal(
    model_names: SharedString,
    model_count: usize,
    auto_start: bool,
    repairing: bool,
    cancelling: bool,
    progress: Option<RepairProgressState>,
    elapsed_ms: f64,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_keep_original: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_stop: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_repair: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let title: SharedString = if auto_start {
        "Repairing Mesh".into()
    } else if model_count == 1 {
        "Repair Mesh".into()
    } else {
        format!("Repair {model_count} Meshes").into()
    };
    let subtitle: SharedString = if auto_start {
        "Fixing geometry issues.".into()
    } else {
        "Fixes geometry issues before you keep working.".into()
    };

    let close = Rc::new(on_close);
    let header_close = close.clone();

    // The close tile is disabled mid-repair, so it drops its handler.
    let close_button = if repairing {
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .w(px(36.0))
            .h(px(36.0))
            .rounded(px(ROUNDED_MD))
            .border_1()
            .border_color(rgb(mix(BORDER_SUBTLE, BLACK, 0.90)))
            .bg(rgb(mix(SURFACE_1, BLACK, 0.92)))
            .cursor_not_allowed()
            .child(icon("x", 16.0, mix(TEXT_MUTED, SURFACE_2, 0.82)))
            .into_any_element()
    } else {
        icon_button(
            ElementId::named_usize("repair-mesh", 0),
            "x",
            IconButtonVariant::Surface,
            IconButtonSize::Md,
        )
        .on_click(move |event, window, cx| header_close(event, window, cx))
        .into_any_element()
    };

    let header = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .border_b(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
        .px(px(20.0))
        .py(px(16.0))
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
                        .w(px(36.0))
                        .h(px(36.0))
                        .rounded(px(ROUNDED_MD))
                        .border_1()
                        .border_color(rgb(mix_b(AMBER_600, BORDER_SUBTLE, 0.45)))
                        .bg(rgb(mix_b(AMBER_600, SURFACE_1, 0.88)))
                        .child(icon("alert-triangle", 16.0, AMBER_600)),
                )
                .child(
                    div()
                        .min_w(px(0.0))
                        .pr(px(8.0))
                        .child(
                            div()
                                .text_size(px(16.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .line_height(px(16.0 * LEADING_TIGHT))
                                .text_color(rgb(TEXT_STRONG))
                                .child(title),
                        )
                        .child(
                            div()
                                .mt(px(2.0))
                                .text_size(px(11.0))
                                .line_height(px(11.0 * LEADING_SNUG))
                                .text_color(rgb(TEXT_MUTED))
                                .child(subtitle),
                        ),
                ),
        )
        .child(close_button);

    let model_card = identity_card(
        if model_count > 1 { "Models" } else { "Model" },
        model_names,
    );

    let middle: Div = match &progress {
        Some(state) => repair_progress_panel(state, elapsed_ms, cancelling),
        None => muted_small_paragraph(
            "Repairs holes, broken surfaces, and other mesh problems.",
        ),
    };

    let warning = warning_strip(if model_count > 1 {
        "The repaired results will replace these models in your current scene. Large or badly \
         damaged meshes can take longer, and some files may still need manual cleanup afterward."
    } else {
        "The repaired result will replace this model in your current scene. Large or badly \
         damaged meshes can take longer, and some files may still need manual cleanup afterward."
    });

    // `Keep Original` / `Stop` / `Stopping…`.
    let keep_label: SharedString = if cancelling {
        "Stopping…".into()
    } else if repairing {
        "Stop".into()
    } else {
        "Keep Original".into()
    };
    let keep = {
        let mut el = div()
            .id("repair-mesh-keep")
            .flex()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .w_full()
            .h(px(36.0))
            .rounded(px(RADIUS_SM))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(SURFACE_1))
            .pt(px(5.0))
            .pb(px(7.0))
            .px(px(11.0))
            .text_size(px(FONT_MD))
            .font_weight(FontWeight::SEMIBOLD)
            .line_height(px(FONT_MD))
            .text_color(rgb(TEXT_STRONG));
        if cancelling {
            el = el
                .opacity(0.6)
                .cursor_not_allowed()
                .child(spinner(14.0))
                .child(keep_label.clone());
        } else {
            el = el
                .cursor_pointer()
                .hover(|style| style.bg(rgb(SURFACE_2)));
            el = if repairing {
                el.on_click(move |event, window, cx| on_stop(event, window, cx))
            } else {
                el.on_click(move |event, window, cx| on_keep_original(event, window, cx))
            };
            el = el.child(keep_label.clone());
        }
        el
    };

    let footer = if auto_start || repairing {
        div().pt(px(4.0)).child(keep)
    } else {
        div()
            .flex()
            .gap(px(8.0))
            .pt(px(4.0))
            .child(div().flex_1().child(keep))
            .child(div().flex_1().child(
                tinted_action(
                    "repair-mesh-repair",
                    "Repair",
                    Some(("wrench", 14.0)),
                    TintedTone::Accent,
                )
                .w_full()
                .on_click(move |event, window, cx| on_repair(event, window, cx)),
            ))
    };

    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .p(px(20.0))
        .child(model_card)
        .child(middle)
        .child(warning)
        .child(footer);

    custom_dialog(DIALOG_LG, header, body)
}

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
    div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .px(px(16.0))
        .bg(rgba(with_alpha(BLACK, 0x99)))
        .child(
            div()
                .w_full()
                .max_w(px(1024.0))
                .max_h(relative(0.88))
                .overflow_hidden()
                .rounded(px(ROUNDED_XL))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .flex()
                .flex_col()
                .shadow(vec![shadow(0.0, 28.0, 64.0, 0.0, with_alpha(BLACK, 0x7a))])
                .child(header)
                .child(body),
        )
}

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

    div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .px(px(16.0))
        .bg(rgba(with_alpha(BLACK, 0x99)))
        .child(
            div()
                .w_full()
                .max_w(px(1024.0))
                .max_h(relative(0.88))
                .overflow_hidden()
                .rounded(px(ROUNDED_XL))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .flex()
                .flex_col()
                .shadow(vec![shadow(0.0, 28.0, 64.0, 0.0, with_alpha(BLACK, 0x7a))])
                .child(header)
                .child(chips)
                .child(body),
        )
}

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

    div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgba(with_alpha(BLACK, 0xb8)))
        .child(
            div()
                .id("slice-metrics-panel")
                .relative()
                .w_full()
                .max_w(px(1500.0))
                .max_h(relative(0.92))
                .overflow_y_scroll()
                .rounded(px(ROUNDED_XL))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .flex()
                .flex_col()
                .child(header)
                .child(body),
        )
}

// ── Formatting helpers ──────────────────────────────────────────────────────

/// `fmt.fixed(value, digits)`.
fn fixed(value: f64, digits: usize) -> String {
    format!("{value:.digits$}")
}

/// `fmt.quantity(value, digits, unit)` — `"{number} {unit}"`.
fn quantity(value: f64, digits: usize, unit: &str) -> String {
    let number = fixed(value, digits);
    // `UNIT_MESSAGES` in `src/utils/unitFormatting.ts`: most units are
    // `'{value} {unit}'`, but `percent` has no space and `perSecond` is `'/s'`.
    match unit {
        "percent" => format!("{number}%"),
        "perSecond" => format!("{number} /s"),
        _ => format!("{number} {unit}"),
    }
}

/// `fmt.number(value)` — grouped integer.
fn grouped(value: f64) -> String {
    let rounded = value.round();
    let negative = rounded < 0.0;
    let digits = format!("{}", rounded.abs() as u64);
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if negative {
        format!("-{out}")
    } else {
        out
    }
}

/// `value.toLocaleString()` for an optional number, or `—`.
fn grouped_option(value: Option<f64>) -> String {
    match value.filter(|value| value.is_finite()) {
        Some(value) => grouped(value),
        None => "—".into(),
    }
}

/// A number without a trailing `.0` — the build-volume labels.
fn num(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// `formatBytes` in `DiagnosticsModal` — `N/A`, `N B`, `N.N KB`, `N.NN MB`,
/// `N.NN GB`.
fn format_bytes(bytes: Option<f64>) -> String {
    let Some(bytes) = bytes.filter(|bytes| bytes.is_finite()) else {
        return "N/A".into();
    };
    let abs = bytes.max(0.0);
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    if abs >= GB {
        quantity(abs / GB, 2, "GB")
    } else if abs >= MB {
        quantity(abs / MB, 2, "MB")
    } else if abs >= KB {
        quantity(abs / KB, 1, "KB")
    } else {
        quantity(abs, 0, "B")
    }
}

/// `formatPercent` in `DiagnosticsModal`.
fn format_percent(value: Option<f64>) -> String {
    match value.filter(|value| value.is_finite()) {
        Some(value) => quantity(value * 100.0, 1, "percent"),
        None => "N/A".into(),
    }
}

/// `formatBytes` in `HistoryDebugModal` — `N B`, `N.N KB`, `N.N MB`.
fn format_bytes_short(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// `formatHistoryDebugTimestamp` — `new Date(ts).toLocaleTimeString()`.
///
/// GPUI has no locale date formatting; the port prints the clock time of the
/// epoch-millisecond value in UTC as `HH:MM:SS` (24-hour).
fn format_timestamp(ts: f64) -> String {
    if !ts.is_finite() {
        return format!("{ts}");
    }
    let total_seconds = (ts / 1000.0).floor() as i64;
    let seconds_of_day = total_seconds.rem_euclid(86_400);
    format!(
        "{:02}:{:02}:{:02}",
        seconds_of_day / 3600,
        (seconds_of_day % 3600) / 60,
        seconds_of_day % 60
    )
}

/// `formatHistoryLabel` from `src/history/formatHistoryLabel.ts`.
fn format_history_label(value: &str) -> String {
    fn title_words(value: &str) -> String {
        value
            .split_whitespace()
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => {
                        format!("{}{}", first.to_uppercase(), chars.as_str().to_lowercase())
                    }
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn prettify_token(value: &str) -> String {
        // `([a-z0-9])([A-Z])` → `$1 $2`, then `_` and `-` become spaces.
        let mut spaced = String::with_capacity(value.len() + 4);
        let chars: Vec<char> = value.chars().collect();
        for (index, ch) in chars.iter().enumerate() {
            let previous_is_lower_or_digit = index > 0
                && (chars[index - 1].is_ascii_lowercase()
                    || chars[index - 1].is_ascii_digit());
            if index > 0 && ch.is_ascii_uppercase() && previous_is_lower_or_digit {
                spaced.push(' ');
            }
            spaced.push(*ch);
        }
        title_words(&spaced.replace('_', " ").replace('-', " "))
    }

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return value.to_string();
    }
    // `^([^:\s]+)\s*:\s*([^\s]+)(?:\s+(.+))?$`
    if let Some(colon) = trimmed.find(':') {
        let domain_raw = &trimmed[..colon];
        let rest = trimmed[colon + 1..].trim_start();
        if !domain_raw.is_empty() && !domain_raw.contains(char::is_whitespace) {
            let mut parts = rest.splitn(2, char::is_whitespace);
            let action_raw = parts.next().unwrap_or("");
            let suffix = parts.next().map(str::trim).filter(|s| !s.is_empty());
            if !action_raw.is_empty() {
                let domain = prettify_token(domain_raw);
                let action = prettify_token(action_raw);
                return match suffix {
                    Some(suffix) => format!("{domain}: {action} {suffix}"),
                    None => format!("{domain}: {action}"),
                };
            }
        }
    }
    prettify_token(trimmed)
}

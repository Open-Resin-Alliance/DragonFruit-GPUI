//! `src/components/organisms/NotificationStack.tsx` and
//! `src/components/organisms/SystemNotificationStack.tsx`, plus the
//! `Toast` / `ToastViewport` / `Spinner` atoms they render (the atoms live in
//! `ui.rs`; this module only composes them).
//!
//! [`notification_stack`] is the editor shell's bottom-centre toast stack: one
//! fixed viewport per toast kind, each mounted by its own guard and faded by its
//! own `visible` flag. [`system_notification_stack`] is the bottom-right card
//! stack fed by `systemNotificationStore`.
//!
//! # PORT DEVIATIONS
//!
//! * **Transitions and keyframes.** `Toast`'s fade-and-rise (`opacity` +
//!   `translateY(8px)`, 220ms), `animate-pulse` on the warning/error icons and
//!   the system stack's `df-fly-in-right` / `df-fly-out-right` /
//!   `df-system-expiry` keyframes have no GPUI equivalent. Every element is
//!   drawn settled: a toast whose `visible` flag is false is painted at
//!   `opacity(0)`, and the expiry bar is drawn full width.
//! * **`z-index`.** `126` (save, printing error) and `125` (the rest), and the
//!   system stack's `z-[130]`, are paint order here: the viewports are children
//!   of the returned wrapper in the source's JSX order, so later ones paint on
//!   top.
//! * **`pointer-events-none`.** GPUI has no such property. The returned wrapper
//!   and the viewports register no hitbox (no handlers on them), so the canvas
//!   keeps its pointer; only the scene-import toast, which the source makes
//!   `pointer-events-auto`, takes clicks.
//! * **`backdrop-filter`.** The system card's `blur(32px) saturate(1.5)` is
//!   omitted; its translucent `--surface-0` fill is kept.
//! * **`aria-live` / `role="alert"` / `role="button"` / `tabIndex`.** GPUI
//!   widgets carry no ARIA attributes. The scene-import toast is focusable and
//!   answers Enter/Space through `on_key_down`, which is what `tabIndex={0}` +
//!   `onKeyDown` did at the source.
//! * **`currentColor`.** An `svg()` element in GPUI does not inherit text
//!   colour, so every icon colour is passed explicitly. Icons inside a toast
//!   use `--text-strong`, the toast's own `color`.
//! * **`Spinner`'s SMIL rotation.** The save toast's `RefreshCw` spinner is
//!   drawn static at `md` (16px); see `ui::spinner_colored`.
//! * **`SystemNotification`'s React nodes.** `title`, `subtitle`, `icon`,
//!   `versionChip` and `action.icon` were `ReactNode`s. Here they are text
//!   (`SharedString`) and a lucide slug (`&'static str`), so the caller passes
//!   the same glyph the source node would have rendered.
//! * **`SystemNotificationAction.onClick` / `onClose`.** They are
//!   `Rc<dyn Fn(..)>` values on the model, invoked by the stack. `onClose`
//!   fires from the stack's dismiss path, exactly where `handleDismiss` calls
//!   `n.onClose?.()`; the caller's `on_dismiss` should therefore only remove
//!   the notification from its store.
//! * **Exit animation.** `EXIT_MS` (200ms) kept removed notifications on screen
//!   while they slid out. With no animation there is no exiting state: the
//!   stack renders exactly the list it is given, and removal is immediate.
//! * **`Toast`'s `animated` mount frame.** The source flips `entered` on the
//!   next animation frame so the transition has an off state to leave. Static
//!   here: `shown == visible`.

use std::rc::Rc;

use gpui::{
    AnyElement, App, BoxShadow, ClickEvent, Div, FontWeight, IntoElement, SharedString, Window,
    div, prelude::*, px, relative, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_core::theme::*;
use gpui_ui::ui;

/// `ToastViewport offset="1.25rem"`.
const TOAST_OFFSET: f32 = 20.0;
/// `ToastViewport offset="4.5rem"` — the printing-error and orientation slots
/// when another toast occupies the base slot.
const TOAST_OFFSET_STACKED: f32 = 72.0;

/// Tailwind's `text-base` (`1rem`); `theme::FONT_LG` is the app's 17px `--font-size-lg`.
const TEXT_BASE: f32 = 16.0;
/// Tailwind's `leading-tight` (1.25), distinct from `--line-height-tight` (1.2).
const LEADING_TIGHT: f32 = 1.25;

/// Tailwind's `gap-1.5`.
const GAP_1_5: f32 = 6.0;
/// Tailwind's `gap-2`.
const GAP_2: f32 = 8.0;
/// Tailwind's `gap-3`.
const GAP_3: f32 = 12.0;
/// Tailwind's `p-3.5` / `px-3.5`.
const PAD_3_5: f32 = 14.0;
/// Tailwind's `pb-2`.
const PAD_B_2: f32 = 8.0;
/// Tailwind's `px-4` / `py-3`.
const PAD_4: f32 = 16.0;
const PAD_Y_3: f32 = 12.0;
/// Tailwind's `px-2.5` / `py-1`.
const PAD_X_2_5: f32 = 10.0;
const PAD_Y_1: f32 = 4.0;
/// Tailwind's `bottom-6` / `right-6`.
const STACK_INSET: f32 = 24.0;
/// Tailwind's `w-[22rem]`.
const CARD_WIDTH: f32 = 352.0;
/// Tailwind's `h-8 w-8`.
const ICON_BOX: f32 = 32.0;
/// Tailwind's `h-4 w-4`.
const ICON_SIZE: f32 = 16.0;
/// Tailwind's `h-1.5` (progress track).
const PROGRESS_HEIGHT: f32 = 6.0;
/// Tailwind's `h-0.5` (expiry bar).
const EXPIRY_HEIGHT: f32 = 2.0;
/// Tailwind's `text-[11px]`.
const TEXT_11: f32 = 11.0;
/// Tailwind's `text-xs` line-height (`calc(1 / 0.75)` → 16px).
const TEXT_XS_LINE: f32 = 16.0;
/// `.ui-button`'s `!h-9`.
const ACTION_HEIGHT: f32 = 36.0;

/// `#000000` — the base of each `color-mix(..., black N%)` in `.ui-button*`.
const BLACK: u32 = 0x000000;

/// `#rrggbb` plus an alpha byte, for `gpui::rgba`.
fn with_alpha(color: u32, alpha: u8) -> u32 {
    (color << 8) | alpha as u32
}

// ── NotificationStack ───────────────────────────────────────────────────────

/// `saveToastMode`: which of the two save-toast labels is showing.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SaveToastMode {
    #[default]
    Saving,
    Autosaving,
}

/// `historyActionToast.direction`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HistoryDirection {
    Undo,
    Redo,
}

/// `historyActionToast`: `{ id, label, direction }`.
#[derive(Clone, Debug)]
pub struct HistoryActionToast {
    pub id: u64,
    /// The resolved Lingui label (`_(historyActionToast.label)`).
    pub label: String,
    pub direction: HistoryDirection,
}

/// `{ id, text }` — the printing-monitor and export-error toasts.
#[derive(Clone, Debug)]
pub struct IdText {
    pub id: u64,
    pub text: String,
}

/// `SceneImportReportTone`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SceneImportTone {
    Success,
    Warning,
    Error,
}

/// `SceneImportReport['clickAction']`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SceneImportClickAction {
    OpenMeshRepairReport,
}

/// `SceneImportReport` from `useSceneCollectionManager`.
#[derive(Clone, Debug)]
pub struct SceneImportReport {
    pub id: u64,
    pub text: String,
    pub tone: SceneImportTone,
    pub click_action: Option<SceneImportClickAction>,
}

/// `exportSuccessToast`: `{ id, path }`.
#[derive(Clone, Debug)]
pub struct ExportSuccessToast {
    pub id: u64,
    /// Where the exported file was written.
    pub path: String,
}

/// `OrientationToastReport['status']`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrientationStatus {
    Applied,
    AlreadyOptimal,
}

/// `OrientationToastReport` from `useEditorToasts`.
#[derive(Clone, Debug)]
pub struct OrientationToast {
    pub id: u64,
    pub status: OrientationStatus,
    /// The display name of the model that was reoriented.
    pub model_name: String,
}

/// `NotificationStackProps`, one field per prop.
///
/// The `is*Visible` flags are the source's `visible` props: they fade a mounted
/// toast (here `opacity`), they do not unmount it. The `is*ToastVisible` guard
/// is the JSX `&&`.
#[derive(Clone, Debug, Default)]
pub struct NotificationStackProps {
    pub is_save_toast_visible: bool,
    pub is_save_toast_animated_visible: bool,
    pub save_toast_mode: SaveToastMode,

    pub history_action_toast: Option<HistoryActionToast>,
    pub is_history_action_toast_visible: bool,

    pub printing_monitor_error_toast: Option<IdText>,
    pub is_printing_monitor_error_toast_visible: bool,

    pub scene_import_report: Option<SceneImportReport>,
    pub is_scene_import_toast_visible: bool,

    pub export_success_toast: Option<ExportSuccessToast>,
    pub is_export_success_toast_visible: bool,

    pub export_error_toast: Option<IdText>,
    pub is_export_error_toast_visible: bool,

    pub orientation_toast: Option<OrientationToast>,
    pub is_orientation_toast_visible: bool,
}

/// A toast's `flex items-center gap-2` body: a 16px icon and the text.
fn toast_content(icon_name: &str, text: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(GAP_2))
        .child(icon(icon_name, ICON_SIZE, TEXT_STRONG))
        .child(text.into())
}

/// A `visible` toast's static equivalent of `Toast`'s animation state.
fn faded(visible: bool) -> f32 {
    if visible { 1.0 } else { 0.0 }
}

/// `NotificationStack`: the editor shell's bottom-centre toast stack.
///
/// `on_open_mesh_repair_report` is the `onOpenMeshRepairReport` prop; it only
/// runs for a scene-import report whose `clickAction` is
/// [`SceneImportClickAction::OpenMeshRepairReport`].
///
/// The returned element is an absolute, window-sized wrapper so the source's
/// `fixed` viewports resolve against the shell; it registers no hitbox.
pub fn notification_stack(
    props: NotificationStackProps,
    on_open_mesh_repair_report: impl Fn(&mut Window, &mut App) + 'static,
) -> Div {
    let open_report: Rc<dyn Fn(&mut Window, &mut App)> = Rc::new(on_open_mesh_repair_report);
    let mut toasts: Vec<AnyElement> = Vec::new();

    // `isSaveToastVisible` — tone info, `Spinner size="md" icon={RefreshCw}`.
    if props.is_save_toast_visible {
        let label = match props.save_toast_mode {
            SaveToastMode::Saving => "Saving…",
            SaveToastMode::Autosaving => "Autosaving…",
        };
        let content = div()
            .flex()
            .items_center()
            .gap(px(GAP_2))
            .child(
                div()
                    .flex_shrink_0()
                    .w(px(ICON_SIZE))
                    .h(px(ICON_SIZE))
                    .child(icon("refresh-cw", ICON_SIZE, TEXT_STRONG)),
            )
            .child(SharedString::from(label));
        toasts.push(
            ui::toast_viewport(
                ui::ToastViewportPosition::BottomCenter,
                TOAST_OFFSET,
                ui::toast(ui::ToastTone::Info, ui::ToastShape::Pill, content)
                    .opacity(faded(props.is_save_toast_animated_visible)),
            )
            .into_any_element(),
        );
    }

    // `historyActionToast` — warning for undo, info for redo.
    if let Some(toast) = &props.history_action_toast {
        let (tone, icon_name) = match toast.direction {
            HistoryDirection::Undo => (ui::ToastTone::Warning, "undo-2"),
            HistoryDirection::Redo => (ui::ToastTone::Info, "redo-2"),
        };
        toasts.push(
            ui::toast_viewport(
                ui::ToastViewportPosition::BottomCenter,
                TOAST_OFFSET,
                ui::toast(
                    tone,
                    ui::ToastShape::Pill,
                    toast_content(icon_name, toast.label.clone()),
                )
                .opacity(faded(props.is_history_action_toast_visible)),
            )
            .into_any_element(),
        );
    }

    // `printingMonitorErrorToast` — error; steps up when another toast owns the
    // base slot (`historyActionToast || sceneImportReport || orientationToast`).
    if let Some(toast) = &props.printing_monitor_error_toast {
        let stacked = props.history_action_toast.is_some()
            || props.scene_import_report.is_some()
            || props.orientation_toast.is_some();
        let offset = if stacked {
            TOAST_OFFSET_STACKED
        } else {
            TOAST_OFFSET
        };
        toasts.push(
            ui::toast_viewport(
                ui::ToastViewportPosition::BottomCenter,
                offset,
                ui::toast(
                    ui::ToastTone::Error,
                    ui::ToastShape::Pill,
                    toast_content("triangle-alert", toast.text.clone()),
                )
                .opacity(faded(props.is_printing_monitor_error_toast_visible)),
            )
            .into_any_element(),
        );
    }

    // `sceneImportReport` — tone from the report; clickable when it opens the
    // mesh repair report.
    if let Some(report) = &props.scene_import_report {
        let tone = match report.tone {
            SceneImportTone::Error => ui::ToastTone::Error,
            SceneImportTone::Warning => ui::ToastTone::Warning,
            SceneImportTone::Success => ui::ToastTone::Success,
        };
        let icon_name = match report.tone {
            SceneImportTone::Success => "circle-check",
            SceneImportTone::Warning | SceneImportTone::Error => "triangle-alert",
        };
        let clickable = report.click_action == Some(SceneImportClickAction::OpenMeshRepairReport);
        let toast = ui::toast(
            tone,
            ui::ToastShape::Pill,
            toast_content(icon_name, report.text.clone()),
        )
        .opacity(faded(props.is_scene_import_toast_visible));
        let toast: AnyElement = if clickable {
            let click = Rc::clone(&open_report);
            let key = Rc::clone(&open_report);
            toast
                .id(format!("scene-import-toast-{}", report.id))
                .cursor_pointer()
                .focusable()
                .tab_stop(true)
                .on_click(move |_, window, cx| click(window, cx))
                .on_key_down(move |event, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        // `event.preventDefault()`: the key does not also
                        // reach the shell's global handler.
                        cx.stop_propagation();
                        key(window, cx);
                    }
                })
                .into_any_element()
        } else {
            toast.into_any_element()
        };
        toasts.push(
            ui::toast_viewport(
                ui::ToastViewportPosition::BottomCenter,
                TOAST_OFFSET,
                toast,
            )
            .into_any_element(),
        );
    }

    // `exportSuccessToast` — `Saved to: {path}`.
    if let Some(toast) = &props.export_success_toast {
        let label = format!("Saved to: {}", toast.path);
        toasts.push(
            ui::toast_viewport(
                ui::ToastViewportPosition::BottomCenter,
                TOAST_OFFSET,
                ui::toast(
                    ui::ToastTone::Success,
                    ui::ToastShape::Pill,
                    toast_content("circle-check", label),
                )
                .opacity(faded(props.is_export_success_toast_visible)),
            )
            .into_any_element(),
        );
    }

    // `exportErrorToast` — error.
    if let Some(toast) = &props.export_error_toast {
        toasts.push(
            ui::toast_viewport(
                ui::ToastViewportPosition::BottomCenter,
                TOAST_OFFSET,
                ui::toast(
                    ui::ToastTone::Error,
                    ui::ToastShape::Pill,
                    toast_content("triangle-alert", toast.text.clone()),
                )
                .opacity(faded(props.is_export_error_toast_visible)),
            )
            .into_any_element(),
        );
    }

    // `orientationToast` — success when applied, info when already optimal;
    // steps up when `historyActionToast || sceneImportReport` owns the base slot.
    if let Some(toast) = &props.orientation_toast {
        let stacked = props.history_action_toast.is_some() || props.scene_import_report.is_some();
        let offset = if stacked {
            TOAST_OFFSET_STACKED
        } else {
            TOAST_OFFSET
        };
        let (tone, icon_name, label) = match toast.status {
            OrientationStatus::Applied => (
                ui::ToastTone::Success,
                "circle-check",
                format!("Successfully oriented {}", toast.model_name),
            ),
            OrientationStatus::AlreadyOptimal => (
                ui::ToastTone::Info,
                "info",
                "Already optimal for this goal.".to_string(),
            ),
        };
        toasts.push(
            ui::toast_viewport(
                ui::ToastViewportPosition::BottomCenter,
                offset,
                ui::toast(
                    tone,
                    ui::ToastShape::Pill,
                    toast_content(icon_name, label),
                )
                .opacity(faded(props.is_orientation_toast_visible)),
            )
            .into_any_element(),
        );
    }

    div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .children(toasts)
}

// ── SystemNotificationStack ─────────────────────────────────────────────────

/// `SystemNotificationTone`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SystemNotificationTone {
    Info,
    Success,
    Warning,
    Error,
    Accent,
    AccentSecondary,
}

/// `SystemNotificationAction['variant']`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SystemActionVariant {
    #[default]
    Secondary,
    Accent,
    AccentSecondary,
    Danger,
}

/// `SystemNotificationAction`: `{ label, icon?, variant?, onClick, closeOnClick? }`.
///
/// `onClick` runs first; the stack then dismisses the notification when
/// `close_on_click` is true, which is the source's
/// `if (action.closeOnClick !== false) onClose?.()`.
#[derive(Clone)]
pub struct SystemNotificationAction {
    pub label: SharedString,
    /// A lucide slug for the source's `icon` node, drawn at 16px in the
    /// button's text colour.
    pub icon: Option<&'static str>,
    pub variant: SystemActionVariant,
    pub close_on_click: bool,
    pub on_click: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
}

impl SystemNotificationAction {
    /// `variant: 'secondary'`, no icon, `closeOnClick` defaulting to true.
    pub fn new(
        label: impl Into<SharedString>,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            icon: None,
            variant: SystemActionVariant::Secondary,
            close_on_click: true,
            on_click: Rc::new(on_click),
        }
    }
}

/// The source's `icon?: ReactNode | null` plus `hideIcon?: boolean`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum NotificationIcon {
    /// `icon` omitted — the tone's glyph.
    #[default]
    Tone,
    /// `icon: null` or `hideIcon: true` — no glyph box at all.
    Hidden,
    /// A lucide slug, drawn in the tone's colour.
    Slug(&'static str),
}

/// `SystemNotification` from `systemNotificationStore`.
#[derive(Clone)]
pub struct SystemNotification {
    pub id: SharedString,
    pub title: SharedString,
    pub subtitle: Option<SharedString>,
    pub tone: Option<SystemNotificationTone>,
    pub icon: NotificationIcon,
    /// `expiryMs`; the countdown bar shows when this is `Some(ms)` and `ms > 0`.
    pub expiry_ms: Option<u64>,
    /// `dismissible !== false`.
    pub dismissible: bool,
    /// `progressPct`; `0..=100`.
    pub progress_pct: Option<f32>,
    pub version_chip: Option<SharedString>,
    /// `onClose`, run by the stack's dismiss path.
    pub on_close: Option<Rc<dyn Fn()>>,
    pub actions: Vec<SystemNotificationAction>,
}

impl SystemNotification {
    /// `tone` unset, tone glyph, dismissible, no actions — the store's defaults.
    pub fn new(id: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            subtitle: None,
            tone: None,
            icon: NotificationIcon::Tone,
            expiry_ms: None,
            dismissible: true,
            progress_pct: None,
            version_chip: None,
            on_close: None,
            actions: Vec::new(),
        }
    }
}

/// `toneStyles(tone)`: the icon chip's `(border, background, text)` plus the
/// glyph the tone supplies when the notification has none.
struct ToneStyle {
    border: u32,
    background: u32,
    color: u32,
    icon: &'static str,
}

/// `color-mix(in srgb, <tone>, <base> N%)` for the chip and the card's tints.
fn tone_style(tone: Option<SystemNotificationTone>) -> ToneStyle {
    // `toneStyles`'s `default` branch covers `info`, `accent-secondary` and an
    // unset tone.
    let (base, icon_name) = match tone {
        Some(SystemNotificationTone::Error) => (DANGER, "triangle-alert"),
        Some(SystemNotificationTone::Warning) => (0xf59e0b, "triangle-alert"),
        Some(SystemNotificationTone::Success) => (SUCCESS, "circle-check"),
        Some(SystemNotificationTone::Accent) => (ACCENT, "cloud-download"),
        Some(SystemNotificationTone::Info)
        | Some(SystemNotificationTone::AccentSecondary)
        | None => (ACCENT_SECONDARY, "cloud-download"),
    };
    ToneStyle {
        // `color-mix(in srgb, <tone>, var(--border-subtle) 40%)`.
        border: mix(base, BORDER_SUBTLE, 0.6),
        // `color-mix(in srgb, <tone>, var(--surface-1) 85%)`.
        background: mix(base, SURFACE_1, 0.15),
        color: base,
        icon: icon_name,
    }
}

/// `ActionButton`: `.ui-button` geometry with `flex-1 !h-9` and the variant's
/// tint. `accent-secondary` has no matching `Button` variant, so it keeps the
/// source's inline tint (which also pins its hover fill).
fn action_button(
    id: impl Into<gpui::ElementId>,
    action: &SystemNotificationAction,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    // The variant already carries `TintedAccent`/`TintedDanger`; only
    // `accent-secondary` has to restate its inline fill, border, text and the
    // hover fill that inline `background` pins in the source.
    let (variant, text, accent_secondary) = match action.variant {
        SystemActionVariant::Secondary => (ui::ButtonVariant::Secondary, TEXT_STRONG, None),
        SystemActionVariant::Accent => (ui::ButtonVariant::TintedAccent, ACCENT, None),
        SystemActionVariant::Danger => (ui::ButtonVariant::TintedDanger, DANGER, None),
        SystemActionVariant::AccentSecondary => {
            let background = mix(ACCENT_SECONDARY, SURFACE_1, 0.14);
            (
                ui::ButtonVariant::TintedAccent,
                ACCENT_SECONDARY,
                Some((
                    background,
                    mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.45),
                )),
            )
        }
    };

    let mut spec = ui::ButtonSpec::new(variant, ui::ButtonSize::Md).gap(GAP_1_5);
    if let Some((background, _)) = accent_secondary {
        spec.hover = Some(background);
    }
    if let Some(icon_name) = action.icon {
        spec = spec.icon(icon_name, ICON_SIZE, text);
    }

    let mut el = ui::button_with_content(id, &spec, action.label.clone()).flex_1();
    if let Some((background, border)) = accent_secondary {
        el = el
            .bg(rgb(background))
            .border_color(rgb(border))
            .text_color(rgb(text));
    }
    el.on_click(on_click)
}

/// One card of `SystemNotificationStack`.
fn system_card(
    notification: &SystemNotification,
    on_dismiss: Rc<dyn Fn(&SharedString, &mut Window, &mut App)>,
) -> AnyElement {
    let tone = tone_style(notification.tone);
    let title_color = match notification.tone {
        Some(SystemNotificationTone::Success) => SUCCESS,
        Some(SystemNotificationTone::AccentSecondary) => ACCENT_SECONDARY,
        _ => TEXT_STRONG,
    };

    // Header: `flex flex-col items-center gap-1.5 p-3.5 text-center`.
    let mut header = div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(GAP_1_5))
        .p(px(PAD_3_5))
        .text_center();

    let glyph = match notification.icon {
        NotificationIcon::Hidden => None,
        NotificationIcon::Tone => Some(tone.icon),
        NotificationIcon::Slug(slug) => Some(slug),
    };
    if let Some(glyph) = glyph {
        header = header.child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .w(px(ICON_BOX))
                .h(px(ICON_BOX))
                .rounded(px(ROUNDED_MD))
                .border_1()
                .border_color(rgb(tone.border))
                .bg(rgb(tone.background))
                .text_color(rgb(tone.color))
                .child(icon(glyph, ICON_SIZE, tone.color)),
        );
    }

    header = header.child(
        div()
            .min_w(px(0.0))
            .text_center()
            .child(
                div()
                    .text_size(px(TEXT_BASE))
                    .line_height(px(TEXT_BASE * LEADING_TIGHT))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(title_color))
                    .text_center()
                    .child(notification.title.clone()),
            )
            .when_some(notification.subtitle.clone(), |this, subtitle| {
                this.child(
                    div()
                        .mt(px(SPACE_1))
                        .text_size(px(FONT_XS))
                        .line_height(px(TEXT_XS_LINE))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(TEXT_MUTED))
                        .text_center()
                        .child(subtitle),
                )
            }),
    );

    // `versionChip`: an accent pill under the header.
    let version_chip = notification.version_chip.clone().map(|chip| {
        div()
            .px(px(PAD_3_5))
            .pb(px(PAD_B_2))
            .mt(px(-SPACE_1))
            .text_center()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .border_1()
                    .border_color(rgb(mix(ACCENT, BORDER_SUBTLE, 0.6)))
                    .bg(rgb(mix(ACCENT, SURFACE_1, 0.15)))
                    .text_color(rgb(ACCENT))
                    .px(px(PAD_X_2_5))
                    .py(px(PAD_Y_1))
                    .text_size(px(TEXT_11))
                    .line_height(px(TEXT_XS_LINE))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(chip),
            )
    });

    // `progressPct`: a 6px track with an accent fill.
    let progress = notification.progress_pct.map(|pct| {
        div().px(px(PAD_3_5)).pb(px(PAD_B_2)).child(
            div()
                .h(px(PROGRESS_HEIGHT))
                .w_full()
                .overflow_hidden()
                .rounded_full()
                .bg(rgb(SURFACE_1))
                .child(
                    div()
                        .h_full()
                        .rounded_full()
                        .bg(rgb(ACCENT))
                        .w(relative(pct / 100.0)),
                ),
        )
    });

    // Footer: `flex items-center gap-3 border-t px-4 py-3`, with the expiry bar
    // pinned to its top edge.
    let mut footer = div()
        .relative()
        .flex()
        .items_center()
        .gap(px(GAP_3))
        .px(px(PAD_4))
        .py(px(PAD_Y_3))
        .overflow_hidden()
        .border_t_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1));

    if notification.expiry_ms.is_some_and(|ms| ms > 0) {
        footer = footer.child(
            div()
                .absolute()
                .left(px(0.0))
                .right(px(0.0))
                .top(px(0.0))
                .h(px(EXPIRY_HEIGHT))
                .overflow_hidden()
                .child(div().h_full().w_full().bg(rgb(ACCENT))),
        );
    }

    if notification.actions.is_empty() {
        if notification.dismissible {
            let dismiss = Rc::clone(&on_dismiss);
            let id = notification.id.clone();
            let on_close = notification.on_close.clone();
            footer = footer.child(
                ui::button(
                    format!("system-dismiss-{}", notification.id),
                    "Dismiss",
                    ui::ButtonVariant::Secondary,
                    ui::ButtonSize::Auto,
                )
                .flex_1()
                .h(px(ACTION_HEIGHT))
                .on_click(move |_, window, cx| {
                    dismiss(&id, window, cx);
                    if let Some(on_close) = &on_close {
                        on_close();
                    }
                }),
            );
        }
    } else {
        for (index, action) in notification.actions.iter().enumerate() {
            let action_click = Rc::clone(&action.on_click);
            let dismiss = Rc::clone(&on_dismiss);
            let id = notification.id.clone();
            let on_close = notification.on_close.clone();
            let close_on_click = action.close_on_click;
            footer = footer.child(action_button(
                format!("system-action-{}-{index}", notification.id),
                action,
                move |event, window, cx| {
                    action_click(event, window, cx);
                    if close_on_click {
                        dismiss(&id, window, cx);
                        if let Some(on_close) = &on_close {
                            on_close();
                        }
                    }
                },
            ));
        }
    }

    // Card: `rounded-xl border shadow-2xl backdrop-blur-2xl`, a translucent
    // `--surface-0` fill and the source's custom shadow.
    div()
        .w(px(CARD_WIDTH))
        .child(
            div()
                .relative()
                .overflow_hidden()
                .rounded(px(ROUNDED_XL))
                .border_1()
                .border_color(rgba(with_alpha(BORDER_SUBTLE, 235)))
                .bg(rgba(with_alpha(SURFACE_0, 224)))
                .shadow(vec![
                    BoxShadow::new(px(0.0), px(18.0), rgba(with_alpha(BLACK, 107)).into())
                        .blur_radius(px(48.0)),
                ])
                .child(header)
                .children(version_chip)
                .children(progress)
                .child(footer),
        )
        .into_any_element()
}

/// `SystemNotificationStack`: the bottom-right card stack.
///
/// `on_dismiss` is `handleDismiss`: it removes the notification from the
/// caller's store. The stack runs the notification's own `on_close` itself, so
/// `on_dismiss` must not.
pub fn system_notification_stack(
    notifications: &[SystemNotification],
    on_dismiss: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
) -> Div {
    let on_dismiss: Rc<dyn Fn(&SharedString, &mut Window, &mut App)> = Rc::new(on_dismiss);
    let mut stack = div()
        .absolute()
        .bottom(px(STACK_INSET))
        .right(px(STACK_INSET))
        .w(px(CARD_WIDTH))
        .flex()
        .flex_col()
        .gap(px(GAP_3));
    for notification in notifications {
        stack = stack.child(system_card(notification, Rc::clone(&on_dismiss)));
    }
    stack
}

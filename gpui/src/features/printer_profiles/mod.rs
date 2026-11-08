//! The `Printer & Material Profiles` modal — the real printer surface of the
//! app (the bar's printer button's `requestOpenProfileSettings('printer')`).
//!
//! Ported from `src/components/settings/ProfileSettingsModal.tsx` (7,493
//! lines): the shell (`max-w-[1120px]`, the accent `Box` tile, `Printer &
//! Material Profiles`, one close tile, no footer), the printer rail
//! ([`list`]), the material panes ([`materials`]) and the nested printer editor
//! ([`editor`]). The model those panes read and write — `PrinterProfile`,
//! `PrinterPreset`, `MaterialProfile` and the store — is [`model`].
//!
//! This module owns the shell and the state that is neither a rail nor a pane:
//! the custom-profile warning, the empty state, the delete confirmation, and
//! the open/close/Escape words the parent calls.
//!
//! # PORT DEVIATIONS
//!
//! 1. The shell is `ui::dialog_modal_full` (as the ticket requires): a 1120px
//!    panel with the shared header/close tile. The source's own header is
//!    `px-3 py-3` with a 32px tile; the shared component is `px-5 py-4` with a
//!    36px tile and also supplies the body's `p-5`, so the source's extra
//!    `px-3 py-3` body padding is not repeated.
//! 2. The source's panel is `h-full` with printers and `h-[700px]` without.
//!    GPUI gives this function no window measurement, so the scrollable body is
//!    the source's only literal height — 700px — in both cases.
//! 3. The source renders the delete confirmation, the tank-mismatch dialog and
//!    the printer editor as portal children of the modal. GPUI has no portal;
//!    they are absolute children of this module's own full-size layer, so they
//!    paint over the modal. The tank-mismatch dialog belongs to the materials
//!    pane, which portals it inside its own subtree.
//! 4. `useEscapeToClose`'s order is kept one level at a time: the delete
//!    confirmation, then the printer editor, then the parent's close. A nested
//!    dialog of the materials pane consumes Escape inside that pane.

pub mod editor;
pub mod list;
pub mod materials;
pub mod model;

use gpui::{
    App, ClickEvent, FontWeight, IntoElement, SharedString, WeakEntity, Window, div,
    linear_color_stop, linear_gradient, prelude::*, px, rgb,
};
use gpui_core::icons::icon;
use gpui_core::theme::*;
use gpui_ui::ui::{
    ButtonSize, ButtonSpec, ButtonVariant, IconTone, button_with, dialog_modal_full,
};

use crate::DragonFruit;
use crate::features::printer_profiles::model::PrinterProfile;

const BLACK: u32 = 0x000000;

/// `#d97706`: the source's warning amber (the custom-profile banner and the
/// delete dialog's icon tone).
const WARNING_AMBER: u32 = 0xd97706;

fn notify(entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    entity.update(cx, |_, cx| cx.notify()).ok();
}

fn close(entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    entity
        .update(cx, |this, cx| {
            this.show_printer_profiles = false;
            cx.notify();
        })
        .ok();
}

/// `requestOpenProfileSettings('printer')`: reset the nested surfaces when the
/// modal opens. The parent calls this beside setting its flag.
pub fn begin() {
    materials::begin();
    model::begin();
}

/// The modal's `useEscapeToClose`. Returns `true` when a nested surface
/// consumed the key; `false` means the parent should close the modal.
///
/// The source unwinds one level at a time: the materials pane's own dialogs
/// (tank mismatch, material editor, lock and delete confirms), then the delete
/// confirmation, then the printer editor.
pub fn escape() -> bool {
    if materials::escape() {
        return true;
    }
    model::escape()
}

// ── The modal ───────────────────────────────────────────────────────────────

/// The `Printer & Material Profiles` modal. The parent keeps the open flag
/// (`show_printer_profiles`) and mounts this after the settings modal, before
/// the printer picker, which paints over it.
pub fn printer_profiles(open: bool, entity: WeakEntity<DragonFruit>) -> impl IntoElement {
    if !open {
        return div();
    }

    let profiles = model::profiles();
    let selected = model::selected_profile();
    let mut column = div().flex().flex_col().gap(px(12.0));

    // `isCustomSelectedPrinter`: a custom, non-official profile.
    if selected
        .as_ref()
        .is_some_and(|profile| profile.is_custom && !profile.is_official)
    {
        column = column.child(custom_profile_warning());
    }

    if profiles.is_empty() {
        column = column.child(empty_state(entity.clone()));
    } else {
        column = column.child(list::printer_section(
            &profiles,
            selected.as_ref(),
            entity.clone(),
        ));
        if let Some(profile) = &selected {
            // `materials.rs` is responsible for the material list and the
            // Multi-Tank section; it reads `model::materials_for`.
            column = column.child(materials::materials_panel(profile.clone(), entity.clone()));
        }
    }

    // `px-3 py-3 custom-scrollbar flex-1 min-h-0 overflow-y-auto`. The shared
    // dialog supplies `p-5`; the fixed height is deviation 2.
    let body = div()
        .id("printer-profiles-body")
        .h(px(700.0))
        .min_h(px(0.0))
        .overflow_y_scroll()
        .child(column);

    let close_entity = entity.clone();
    let mut layer = div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .child(dialog_modal_full(
            1120.0,
            "Printer & Material Profiles",
            None,
            Some("box"),
            IconTone::Accent,
            false,
            body,
            // The source's modal has no footer action row: `Close` is the
            // header tile. An empty row keeps `dialog_modal_full`'s signature.
            div(),
            move |_, _, cx| close(&close_entity, cx),
        ));

    if let Some((_, name)) = model::pending_remove() {
        layer = layer.child(delete_confirm(name, entity.clone()));
    }

    if model::editing() {
        if let Some(profile) = model::selected_profile() {
            layer = layer.child(editor::printer_editor(profile, entity.clone()));
        }
    }

    layer
}

/// The custom-profile safety banner (`isCustomSelectedPrinter`).
fn custom_profile_warning() -> impl IntoElement {
    div()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(mix(WARNING_AMBER, BORDER_SUBTLE, 0.70)))
        .bg(rgb(mix(WARNING_AMBER, SURFACE_1, 0.08)))
        .px(px(12.0))
        .py(px(8.0))
        .flex()
        .items_start()
        .gap(px(8.0))
        .text_size(px(FONT_XS))
        .line_height(px(FONT_XS * 1.45))
        .text_color(rgb(TEXT_MUTED))
        .child(
            div()
                .flex_shrink_0()
                .mt(px(2.0))
                .child(icon("triangle-alert", 16.0, WARNING_AMBER)),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Safety warning: "),
                )
                .child(
                    "Custom, non-official profiles may increase the risk of print failure and \
                     can potentially damage the machine or cause personal injury. Verify all \
                     settings carefully before printing.",
                ),
        )
}

/// `!hasPrinters`: the `Welcome to Printer Profiles` card.
fn empty_state(entity: WeakEntity<DragonFruit>) -> impl IntoElement {
    div()
        .flex_1()
        .h_full()
        .min_h(px(0.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(linear_gradient(
            180.0,
            linear_color_stop(rgb(mix(SURFACE_2, SURFACE_0, 0.96)), 0.0),
            linear_color_stop(rgb(mix(SURFACE_2, BLACK, 0.92)), 1.0),
        ))
        .px(px(16.0))
        .py(px(40.0))
        .child(
            div()
                .max_w(px(520.0))
                .flex()
                .flex_col()
                .items_center()
                .text_center()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(56.0))
                        .h(px(56.0))
                        .mb(px(12.0))
                        .rounded_full()
                        .border_1()
                        .border_color(rgb(mix(ACCENT, BORDER_SUBTLE, 0.78)))
                        .bg(rgb(mix(ACCENT, SURFACE_1, 0.12)))
                        .child(icon("printer", 24.0, ACCENT)),
                )
                .child(
                    div()
                        .text_size(px(22.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(px(22.0 * 1.25))
                        .text_color(rgb(TEXT_STRONG))
                        .child("Welcome to Printer Profiles"),
                )
                .child(
                    div()
                        .mt(px(8.0))
                        .text_size(px(FONT_MD))
                        .line_height(px(FONT_MD * 1.625))
                        .text_color(rgb(TEXT_MUTED))
                        .child(
                            "Add your first printer from the library to unlock a tailored \
                             materials list and printer-specific defaults.",
                        ),
                )
                .child(div().mt(px(20.0)).child(list::add_printer_button(entity))),
        )
}

/// `deleteConfirmTarget` for `kind === 'printer'`: `Delete Printer Profile`.
fn delete_confirm(name: String, entity: WeakEntity<DragonFruit>) -> impl IntoElement {
    let cancel_entity = entity.clone();
    let confirm_entity = entity.clone();
    let close_entity = entity.clone();

    dialog_modal_full(
        448.0, // max-w-md
        "Delete Printer Profile",
        Some("This action cannot be undone.".into()),
        Some("triangle-alert"),
        IconTone::Warning,
        false,
        div()
            .flex()
            .flex_wrap()
            .text_size(px(FONT_XS))
            .line_height(px(FONT_XS * 1.375))
            .text_color(rgb(TEXT_MUTED))
            .child("Delete ")
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(TEXT_STRONG))
                    .child(SharedString::from(name)),
            )
            .child(" and remove all material profiles linked to it?"),
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                button_with(
                    "printer-delete-cancel",
                    "Cancel",
                    ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Md),
                )
                .on_click(move |_, _, cx| {
                    model::cancel_remove();
                    notify(&cancel_entity, cx);
                }),
            )
            .child(
                button_with(
                    "printer-delete-confirm",
                    "Delete",
                    ButtonSpec::new(ButtonVariant::TintedDanger, ButtonSize::Md)
                        .gap(6.0)
                        .icon("trash-2", 14.0, DANGER),
                )
                .on_click(move |_, _, cx| {
                    model::confirm_remove();
                    // `handleConfirmDelete` (`:3448-3468`): removing the last
                    // printer profile closes the modal, so the first-run
                    // "Welcome" onboarding can appear instead of an empty
                    // modal.
                    if model::profiles().is_empty() {
                        close(&confirm_entity, cx);
                    }
                    notify(&confirm_entity, cx);
                }),
            ),
        move |_, _, cx| {
            model::cancel_remove();
            notify(&close_entity, cx);
        },
    )
}

/// The profile the editor pane and the materials pane are working on. Exposed
/// so both can be exercised without reaching for the store's selection.
#[allow(dead_code)]
pub fn selected_profile() -> Option<PrinterProfile> {
    model::selected_profile()
}

/// The source's card click callback (`handlePickPrinter`) for callers outside
/// this module, e.g. a bar menu that selects a profile before opening.
#[allow(dead_code)]
pub fn select(id: &str, entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    model::select(id);
    notify(entity, cx);
}

/// Notify the window after a store write made by a pane.
#[allow(dead_code)]
pub fn changed(entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    notify(entity, cx);
}

/// The click handler shape a pane can use for `model::open_editor`
/// (`handlePickPrinter` + `setIsEditingPrinter(true)`).
#[allow(dead_code)]
pub fn open_editor_handler(
    id: String,
    entity: WeakEntity<DragonFruit>,
) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    move |_, _, cx| {
        model::open_editor(&id);
        notify(&entity, cx);
    }
}

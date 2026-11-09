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
//! 1. `backdrop-blur-sm` and the `ui-modal-*` enter transitions have no GPUI
//!    equivalent and are omitted; `bg-black/58` is `rgba(BLACK, 0x94)`.
//! 2. The source's `grid-cols-5` rail is a wrapping flex row of fixed 190px
//!    cards (see [`list`]).
//! 3. The source renders the delete confirmation, the tank-mismatch dialog and
//!    the printer editor as portal children of the modal. GPUI has no portal;
//!    they are absolute children of this module's own full-size layer, so they
//!    paint over the modal. The material pane's dialogs are mounted here for
//!    the same reason — inside the pane their `fixed inset-0` backdrop would be
//!    the pane's box, not the window.
//! 4. `useEscapeToClose`'s order is kept one level at a time: the delete
//!    confirmation, then the printer editor, then the parent's close. A nested
//!    dialog of the materials pane consumes Escape inside that pane.

pub mod editor;
pub mod list;
pub mod materials;
pub mod model;

use gpui::{
    App, ClickEvent, FontWeight, IntoElement, SharedString, WeakEntity, Window, div,
    linear_color_stop, linear_gradient, prelude::*, px, relative, rgb, rgba,
};
use gpui_core::icons::icon;
use gpui_core::theme::*;
use gpui_ui::panel_fields::with_alpha;
use gpui_ui::ui::{
    ButtonSize, ButtonSpec, ButtonVariant, IconButtonSize, IconButtonVariant, IconTone,
    button_with, dialog_modal_full, icon_button,
};

use crate::DragonFruit;
use crate::app::automation;
use crate::features::printer_profiles::model::PrinterProfile;

const BLACK: u32 = 0x000000;

/// A CSS `box-shadow` layer (`editor.rs`'s helper).
fn shadow(x: f32, y: f32, blur: f32, spread: f32, color: u32) -> gpui::BoxShadow {
    gpui::BoxShadow::new(px(x), px(y), rgba(color).into())
        .blur_radius(px(blur))
        .spread_radius(px(spread))
}

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

/// The ids this shell measures for `automation`'s `rects` map.
const MODAL_CARD: &[&str] = &["profile-modal-card"];
const MODAL_PARTS: &[&str] = &["profile-modal-header", "profile-modal-body"];

/// The `Printer & Material Profiles` modal. The parent keeps the open flag
/// (`show_printer_profiles`) and mounts this after the settings modal, before
/// the printer picker, which paints over it.
///
/// `viewport_height` is the window's content height; the source is
/// `ProfileSettingsModal.tsx:3919`: overlay `fixed inset-0 flex items-stretch
/// justify-center bg-black/58 backdrop-blur-sm p-5`, card `w-full
/// max-w-[1120px] flex flex-col rounded-xl border shadow-2xl overflow-hidden`
/// with `h-full` while the store has printers and `self-center h-[700px]
/// max-h-[94vh]` otherwise. There is no footer: the header tile is `Close`.
pub fn printer_profiles(
    open: bool,
    viewport_height: f32,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    if !open {
        // A closed modal must not leave its last frame's boxes assertable.
        automation::clear_rects();
        return div();
    }
    automation::begin_frame();

    let profiles = model::profiles();
    let has_printers = !profiles.is_empty();
    let selected = model::selected_profile();
    let mut column = div().flex().flex_col().gap(px(12.0)).w_full();
    if has_printers {
        // `w-full min-h-full`: the scroller's own height is the floor, so a
        // short pane scrolls no further than the card.
        column = column.min_h(relative(1.0));
    } else {
        // `w-full h-full min-h-0`: the empty state owns the leftover height.
        column = column.h_full().min_h(px(0.0));
    }

    // `isCustomSelectedPrinter`: a custom, non-official profile.
    if selected
        .as_ref()
        .is_some_and(|profile| profile.is_custom && !profile.is_official)
    {
        column = column.child(custom_profile_warning());
    }

    if has_printers {
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
    } else {
        column = column.child(empty_state(entity.clone()));
    }

    // `px-3 py-3 custom-scrollbar flex-1 min-h-0 overflow-y-auto flex`: the
    // body is the card's only shrinking child, so overflow scrolls here
    // instead of growing the card past its height. `track_scroll` publishes
    // the offset so a scenario can click a target below the fold.
    let scroll = automation::body_scroll();
    let body = div()
        .id("printer-profiles-body")
        .track_scroll(&scroll)
        .flex_1()
        .min_h(px(0.0))
        .px(px(12.0))
        .py(px(12.0))
        .flex()
        // `items-start`: the source's inner column is `h-auto min-h-full`, and
        // CSS's stretch would still let the scroller see the column's
        // overflowing content. GPUI measures a scroll container's content as
        // the union of its *direct* children's boxes (`div.rs:2027`), so a
        // stretched column reports the body's own height and the scroll range
        // collapses to zero. Starting the column at the top keeps it
        // content-sized and the body scrollable.
        .items_start()
        .when(has_printers, |body| {
            body.overflow_y_scroll().overflow_x_hidden()
        })
        .when(!has_printers, |body| body.overflow_hidden())
        .child(column);

    // `flex items-center justify-between px-3 py-3 border-b`, tinted
    // `color-mix(in srgb, var(--surface-1), transparent 8%)`.
    let close_entity = entity.clone();
    let header = div()
        .flex()
        .items_center()
        .justify_between()
        .px(px(12.0))
        .py(px(12.0))
        .border_b(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(mix(SURFACE_1, SURFACE_0, 0.92)))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(
                    // `inline-flex h-8 w-8 rounded-md border` over the accent
                    // → accent-secondary gradient.
                    div()
                        .flex()
                        .flex_shrink_0()
                        .items_center()
                        .justify_center()
                        .w(px(32.0))
                        .h(px(32.0))
                        .rounded(px(ROUNDED_MD))
                        .border_1()
                        .border_color(rgb(BORDER_SUBTLE))
                        .bg(linear_gradient(
                            135.0,
                            linear_color_stop(rgb(mix(ACCENT, SURFACE_1, 0.86)), 0.0),
                            linear_color_stop(rgb(mix(ACCENT_SECONDARY, SURFACE_1, 0.90)), 1.0),
                        ))
                        .child(icon("box", 16.0, ACCENT)),
                )
                .child(
                    // `text-base` = 16px (the theme's `FONT_LG` is 17).
                    div()
                        .text_size(px(16.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Printer & Material Profiles"),
                ),
        )
        .child(
            icon_button(
                "printer-profiles-close",
                "x",
                IconButtonVariant::Surface,
                IconButtonSize::Sm,
            )
            .on_click(move |_, _, cx| close(&close_entity, cx)),
        );

    let mut card = div()
        .w_full()
        .max_w(px(1120.0))
        .flex()
        .flex_col()
        .rounded(px(ROUNDED_XL))
        .border_1()
        .border_color(rgb(BORDER_STRONG))
        .bg(rgb(SURFACE_0))
        .overflow_hidden()
        // `0 26px 54px rgba(0,0,0,0.48)`.
        .shadow(vec![shadow(0.0, 26.0, 54.0, 0.0, with_alpha(BLACK, 0x7a))])
        .child(header)
        .child(body);
    card = if has_printers {
        // `h-full`: the padded overlay's content height.
        card.h_full()
    } else {
        // `self-center h-[700px] max-h-[94vh]`.
        card.self_center()
            .h(px(700.0))
            .max_h(px(viewport_height * 0.94))
    };

    let card = automation::measure_children(card, MODAL_PARTS);
    let overlay = div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .flex()
        .items_stretch()
        .justify_center()
        .p(px(20.0))
        // `bg-black/58`.
        .bg(rgba(with_alpha(BLACK, 0x94)))
        .child(card);
    let mut layer = div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .child(automation::measure_children(overlay, MODAL_CARD));

    if let Some((_, name)) = model::pending_remove() {
        layer = layer.child(delete_confirm(name, entity.clone()));
    }

    if model::editing() {
        if let Some(profile) = model::selected_profile() {
            layer = layer.child(editor::printer_editor(
                profile,
                viewport_height,
                entity.clone(),
            ));
        }
    }

    // The material pane's own dialogs are `fixed inset-0` portals in the
    // source; mounted here, their backdrop is the window rather than the
    // pane's own box.
    if let Some(profile) = &selected {
        layer = layer.child(materials::overlays(profile, viewport_height, &entity));
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
                .child(automation::measure_children(
                    div().mt(px(20.0)).child(list::add_printer_button(entity)),
                    &["printer-profiles-add"],
                )),
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

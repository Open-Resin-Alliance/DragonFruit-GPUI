//! The `Printer & Material Profiles` modal's printer rail: the `3D Printer`
//! section with its move-left / move-right / `Add printer` controls, the card
//! grid and the `Edit` / `Remove` band.
//!
//! Ported from `ProfileSettingsModal.tsx` (`renderPrinterRailCard` at 1670, the
//! rail header at 4010, the grid at 4080 and the action band at 4430). The
//! source draws the cards with `grid-cols-5`; GPUI has no CSS grid, so the rail
//! is a wrapping flex row of fixed 190px cards — the same card count per row at
//! the panel's 1120px width, and the source's own `pr-1`/`max-h-[392px]`
//! scrolling once the rail holds more than eight printers.
//!
//! # PORT DEVIATIONS
//!
//! 1. No drag-and-drop: the source reorders by dragging a card
//!    (`onDragStart` / `onDrop`) and this GPUI build has no drag payload. The
//!    header's `Move selected printer left/right` buttons are the reorder
//!    controls, exactly as in the source; the drag handlers are omitted.
//! 2. No double-click: `ClickEvent` carries no click count here, so the
//!    source's double-click-to-edit is the `Edit` button in the action band.
//! 3. Printer artwork is not part of the GPUI asset set (the picker's
//!    deviation 4), so a card always draws the source's own fallback —
//!    `Printer` for a generic machine, `ImagePlus` for the rest.
//! 4. `Import` / `Export` (the printer bundle payload) and `Network Settings`
//!    / `Manage Fleet` have no port: the bundle IO and the network panes are
//!    not modelled here. `Edit` and `Remove` — the two the rail's own flow
//!    needs — are kept.
//! 5. `hover:` fills, `transition-all`, `cursor-grab` and the `dragging`
//!    opacity are not expressible; the selected card keeps the accent fill and
//!    border.

use gpui::{
    App, ClickEvent, Div, FontWeight, IntoElement, Stateful, WeakEntity, Window, div,
    linear_color_stop, linear_gradient, prelude::*, px, rgb,
};
use gpui_core::icons::icon;
use gpui_core::theme::*;
use gpui_ui::ui::{
    ButtonSize, ButtonSpec, ButtonVariant, button_with, button_with_content, fade_text,
};

use crate::DragonFruit;
use crate::features::printer_profiles::model::{self, PrinterProfile};

const WHITE: u32 = 0xffffff;
const BLACK: u32 = 0x000000;

/// `printerImageWellBackground` in the dark theme.
const IMAGE_WELL: u32 = 0x1c2027;

/// Source `accentSecondaryActionStyle93`.
fn accent_secondary_fill() -> (u32, u32, u32) {
    (
        mix(ACCENT_SECONDARY, SURFACE_1, 0.07),
        mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.58),
        ACCENT_SECONDARY,
    )
}

fn notify(entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    entity.update(cx, |_, cx| cx.notify()).ok();
}

/// `handlePickPrinter` on a card.
fn select_handler(
    id: String,
    entity: WeakEntity<DragonFruit>,
) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    move |_, _, cx| {
        model::select(&id);
        notify(&entity, cx);
    }
}

/// The rail's `3D Printer` section.
pub(super) fn printer_section(
    profiles: &[PrinterProfile],
    selected: Option<&PrinterProfile>,
    entity: WeakEntity<DragonFruit>,
) -> Div {
    let index = selected.and_then(|selected| {
        profiles
            .iter()
            .position(|profile| profile.id == selected.id)
    });
    let can_move_left = index.is_some_and(|index| index > 0);
    let can_move_right =
        index.is_some_and(|index| index + 1 < profiles.len());

    let header = div()
        .p(px(12.0))
        .border_b_1()
        .border_color(rgb(BORDER_SUBTLE))
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap(px(8.0))
                .child(
                    div()
                        .min_w(px(0.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .text_size(px(FONT_MD))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child(icon("box", 16.0, TEXT_STRONG))
                                .child("3D Printer"),
                        )
                        .child(
                            div()
                                .mt(px(2.0))
                                .text_size(px(FONT_XS))
                                .text_color(rgb(TEXT_MUTED))
                                .child(
                                    "Each printer can store its own image and has a dedicated \
                                     set of compatible material profiles.",
                                ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .flex_shrink_0()
                        .child(move_button(
                            "printer-move-left",
                            "chevron-left",
                            can_move_left,
                            -1,
                            entity.clone(),
                        ))
                        .child(move_button(
                            "printer-move-right",
                            "chevron-right",
                            can_move_right,
                            1,
                            entity.clone(),
                        ))
                        .child(add_printer_button(entity.clone())),
                ),
        );

    let mut cards = div().flex().flex_wrap().gap(px(10.0));
    for profile in profiles {
        cards = cards.child(printer_card(
            profile,
            selected.is_some_and(|selected| selected.id == profile.id),
            entity.clone(),
        ));
    }

    let mut grid = div().flex().flex_col();
    if profiles.len() > 8 {
        // `shouldConstrainPrinterRailHeight`: `max-h-[392px] overflow-y-auto pr-1`.
        grid = grid.child(
            div()
                .id("printer-rail-grid")
                .max_h(px(392.0))
                .min_h(px(0.0))
                .overflow_y_scroll()
                .pr(px(4.0))
                .child(cards),
        );
    } else {
        grid = grid.child(cards);
    }
    grid = grid.child(action_band(selected, entity.clone()));

    // `rounded-md border overflow-hidden shrink-0` on the source's
    // `linear-gradient(180deg, surface-1 8% transparent, surface-1)` band.
    div()
        .flex_shrink_0()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .overflow_hidden()
        .bg(linear_gradient(
            180.0,
            linear_color_stop(rgb(mix(SURFACE_1, SURFACE_0, 0.92)), 0.0),
            linear_color_stop(rgb(SURFACE_1), 1.0),
        ))
        .child(header)
        .child(div().p(px(12.0)).child(grid))
}

/// `!h-8 !w-8` secondary tile, muted glyph, `disabled:opacity-45`.
fn move_button(
    id: &'static str,
    icon_name: &'static str,
    enabled: bool,
    direction: i32,
    entity: WeakEntity<DragonFruit>,
) -> Stateful<Div> {
    let mut spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Auto)
        .icon_tile(32.0)
        .radius(ROUNDED_SM)
        .fill(SURFACE_1, BORDER_SUBTLE, TEXT_MUTED);
    if !enabled {
        spec = spec.disabled();
    }
    let button = button_with_content(id, spec, icon(icon_name, 14.0, TEXT_MUTED));
    if !enabled {
        return button.opacity(0.45);
    }
    let entity = entity.clone();
    button.on_click(move |_, _, cx| {
        if model::move_selected(direction) {
            notify(&entity, cx);
        }
    })
}

/// `handleAddPrinter`: clear the library selection, then open the printer
/// picker (`showPresetPicker` in the source).
pub(super) fn add_printer_button(entity: WeakEntity<DragonFruit>) -> Stateful<Div> {
    let (background, border, text) = accent_secondary_fill();
    let spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Auto)
        .radius(ROUNDED_SM)
        .padding_x(12.0)
        .padding_y(0.0, 0.0)
        .font_size(14.0)
        .gap(4.0)
        .icon("plus", 14.0, ACCENT_SECONDARY)
        .fill(background, border, text);
    button_with("printer-profiles-add", "Add printer", spec)
        .h(px(32.0))
        .on_click(move |_, _, cx| {
            // The library sheet is the add flow; it paints over this modal.
            crate::features::printer_picker::begin();
            entity
                .update(cx, |this, cx| {
                    this.show_printer_picker = true;
                    cx.notify();
                })
                .ok();
        })
}

/// `renderPrinterRailCard` (`ProfileSettingsModal.tsx:1670`).
fn printer_card(
    profile: &PrinterProfile,
    active: bool,
    entity: WeakEntity<DragonFruit>,
) -> Stateful<Div> {
    let mut art = div()
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .h(px(128.0))
        .w_full()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(IMAGE_WELL))
        .overflow_hidden();
    art = art.child(
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(4.0))
            .px(px(8.0))
            .text_size(px(11.0))
            .line_height(px(11.0 * 1.2))
            .text_color(rgb(TEXT_MUTED))
            .child(icon(
                if profile.is_generic() {
                    "printer"
                } else {
                    "image-plus"
                },
                20.0,
                TEXT_MUTED,
            ))
            .child(if profile.is_generic() {
                "Generic"
            } else {
                "No image"
            }),
    );

    if let Some(text) = profile.card_badge_text() {
        // `hasOfficialUpdate` is not ported, so the chip is `CUSTOM` or the
        // preset's platform badge.
        let color = if profile.is_custom {
            DANGER
        } else {
            profile
                .platform_badge
                .as_ref()
                .and_then(|badge| badge.color.as_deref())
                .and_then(parse_hex_color)
                .unwrap_or(0x0ea5e9)
        };
        art = art.child(
            div()
                .absolute()
                .top(px(4.0))
                .right(px(4.0))
                .flex()
                .items_center()
                .justify_center()
                .h(px(18.0))
                .min_w(px(44.0))
                .rounded(px(ROUNDED_MD))
                .px(px(6.0))
                .text_size(px(9.0))
                .font_weight(FontWeight::BOLD)
                .line_height(px(9.0))
                .text_color(rgb(WHITE))
                .bg(linear_gradient(
                    135.0,
                    linear_color_stop(rgb(mix(color, WHITE, 0.86)), 0.0),
                    linear_color_stop(rgb(mix(color, BLACK, 0.82)), 1.0),
                ))
                .child(text),
        );
    }

    // `bottomRightBadge`: the bit-depth chip, drawn only for a non-8-bit depth.
    if let Some(label) = profile.bit_depth_label() {
        let bits = profile.bit_depth_bits().unwrap_or(0);
        let color = if bits == 3 { DANGER } else { ACCENT_SECONDARY };
        art = art.child(
            div()
                .absolute()
                .bottom(px(4.0))
                .right(px(4.0))
                .flex()
                .items_center()
                .justify_center()
                .h(px(18.0))
                .rounded(px(ROUNDED_MD))
                .border_1()
                .border_color(rgb(mix(color, WHITE, 0.80)))
                .px(px(6.0))
                .text_size(px(9.0))
                .font_weight(FontWeight::BOLD)
                .line_height(px(9.0))
                .text_color(rgb(0xf8fafc))
                .bg(linear_gradient(
                    135.0,
                    linear_color_stop(rgb(mix(color, 0x111827, 0.56)), 0.0),
                    linear_color_stop(rgb(mix(color, 0x0b1220, 0.72)), 1.0),
                ))
                .child(label),
        );
    }

    // `imageOverlay`: the source draws a wifi tile for a connected profile.
    if profile
        .network_connection
        .as_ref()
        .is_some_and(|connection| connection.connected)
    {
        art = art.child(
            div()
                .absolute()
                .top(px(4.0))
                .left(px(4.0))
                .flex()
                .items_center()
                .justify_center()
                .w(px(28.0))
                .h(px(28.0))
                .rounded_full()
                .border_1()
                .border_color(rgb(mix(0x22c55e, WHITE, 0.88)))
                .bg(rgb(mix(0x22c55e, 0x0f172a, 0.40)))
                .child(icon("wifi", 14.0, 0xdcfce7)),
        );
    }

    let id = profile.id.clone();
    let subtitle = profile
        .manufacturer
        .clone()
        .filter(|manufacturer| !manufacturer.trim().is_empty())
        .unwrap_or_else(|| "Generic".to_owned());

    div()
        .id(format!("printer-rail-{}", profile.id))
        .w(px(190.0))
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(if active {
            mix(ACCENT, BORDER_SUBTLE, 0.72)
        } else {
            BORDER_SUBTLE
        }))
        .bg(rgb(if active {
            mix(ACCENT, SURFACE_1, 0.12)
        } else {
            SURFACE_2
        }))
        .p(px(8.0))
        .cursor_pointer()
        .child(art)
        .child(
            div()
                .mt(px(2.0))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(6.0))
                .child(
                    div()
                        .min_w(px(0.0))
                        .flex_1()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(12.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .line_height(px(15.0))
                                .text_color(rgb(TEXT_STRONG))
                                .child(fade_text(profile.name.clone())),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(rgb(TEXT_MUTED))
                                .child(fade_text(subtitle)),
                        ),
                ),
        )
        .on_click(select_handler(id, entity))
}

/// The band under the grid (`ProfileSettingsModal.tsx:4430`).
fn action_band(
    selected: Option<&PrinterProfile>,
    entity: WeakEntity<DragonFruit>,
) -> Div {
    let has_printers = selected.is_some();
    let edit_entity = entity.clone();
    let remove_entity = entity.clone();
    let selected_id = selected.map(|profile| profile.id.clone());

    let edit = if let Some(id) = selected_id.clone() {
        button_with(
            "printer-profiles-edit",
            "Edit",
            ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Auto)
                .radius(ROUNDED_SM)
                .padding_x(12.0)
                .padding_y(0.0, 0.0)
                .font_size(14.0)
                .gap(6.0)
                .icon("pencil", 14.0, TEXT_STRONG),
        )
        .h(px(32.0))
        .on_click(move |_, _, cx| {
            model::open_editor(&id);
            notify(&edit_entity, cx);
        })
    } else {
        button_with(
            "printer-profiles-edit",
            "Edit",
            ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Auto)
                .radius(ROUNDED_SM)
                .padding_x(12.0)
                .padding_y(0.0, 0.0)
                .font_size(14.0)
                .icon("pencil", 14.0, TEXT_MUTED)
                .disabled(),
        )
        .h(px(32.0))
        .opacity(0.45)
        .cursor_not_allowed()
    };

    let mut remove = button_with(
        "printer-profiles-remove",
        "Remove",
        ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Auto)
            .radius(ROUNDED_SM)
            .padding_x(12.0)
            .padding_y(0.0, 0.0)
            .font_size(14.0)
            .gap(6.0)
            .icon("trash-2", 14.0, DANGER),
    )
    .h(px(32.0))
    .ml_auto();
    if has_printers {
        remove = remove.on_click(move |_, _, cx| {
            model::request_remove_selected();
            notify(&remove_entity, cx);
        });
    } else {
        remove = remove
            .text_color(rgb(TEXT_MUTED))
            .opacity(0.45)
            .cursor_not_allowed();
    }

    // `mt-2.5 rounded-md border p-2` on `color-mix(surface-2, transparent 8%)`.
    div()
        .mt(px(10.0))
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(mix(SURFACE_2, SURFACE_1, 0.92)))
        .p(px(8.0))
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(8.0))
                .child(edit)
                .child(remove),
        )
}

/// `sanitizePlatformBadge`'s `#rrggbb` reader (the badge colour the preset
/// JSON carries).
fn parse_hex_color(value: &str) -> Option<u32> {
    let hex = value.trim().strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

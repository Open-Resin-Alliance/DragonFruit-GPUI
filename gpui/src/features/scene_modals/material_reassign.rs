//! `SceneMaterialReassignModal.tsx`.

use std::rc::Rc;

use gpui::{App, ClickEvent, Div, ElementId, SharedString, Window, div, prelude::*, px, rgb};
use gpui_core::theme::*;
use gpui_ui::ui::{
    ButtonSize, ButtonVariant, IconTone, SelectOption, button, dialog_modal, select_dropdown,
};

use super::chrome::{LEADING_RELAXED, AMBER_500, DIALOG_XL, shadow, muted_paragraph, Run, rich_paragraph, action_slot, table_header_cell, table_cell, table_frame, table_head, table_row};

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

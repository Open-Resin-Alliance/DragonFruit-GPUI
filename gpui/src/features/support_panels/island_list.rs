use gpui::{
    App, ClickEvent, Div, FontWeight, IntoElement, Stateful, WeakEntity, Window, div, prelude::*,
    px, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_core::support_state::*;
use gpui_core::theme::*;
use gpui_ui::panel_fields::{BLACK, WHITE, format_number, with_alpha};
use gpui_ui::ui::{ButtonSize, ButtonVariant, button};

use super::shared::{
    GREEN_400, GREEN_500, ORANGE_400, ORANGE_500, ROUNDED_BARE, checkbox, flat_button, notify,
    ui_meta,
};
use crate::DragonFruit;

// ---------------------------------------------------------------------------
// Island list — `IslandListCard.tsx`
// ---------------------------------------------------------------------------
/// PORT GAP: the Hierarchy button sets `show_hierarchy`; `IslandHierarchyModal`
/// is not ported.
pub fn island_list_body(
    state: &IslandListState,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let model = state.model();
    let total = model.islands.len();
    let active = model.active_count();
    let show_merged = model.show_merged;
    let sort_by = model.sort_by();
    let selected_id = model.selected_id;
    let rows = model.rows();
    let search = model.search.clone();
    let layer_height_mm = model.layer_height_mm;
    let z_offset_mm = model.z_offset_mm;
    drop(model);

    // Header right slot: `Button variant="accent" size="sm"` at `!h-9 !px-2.5
    // !py-0 text-[12px]` with an `sm` IconChip and a chevron.
    let hierarchy_state = state.clone();
    let hierarchy_entity = entity.clone();
    let hierarchy = flat_button(
        "island-list-hierarchy",
        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(
                div()
                    .w(px(20.0))
                    .h(px(20.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(ROUNDED_BARE))
                    .border_1()
                    .border_color(rgba(with_alpha(ACCENT_SECONDARY_CONTRAST, 0x8c)))
                    .bg(rgba(with_alpha(ACCENT_SECONDARY_CONTRAST, 0x24)))
                    .child(icon("network", 14.0, ACCENT_SECONDARY_CONTRAST)),
            )
            .child(
                div()
                    .line_height(px(FONT_MD))
                    .text_color(rgb(ACCENT_SECONDARY_CONTRAST))
                    .child("Hierarchy"),
            )
            .child(icon(
                "chevron-right",
                14.0,
                mix(ACCENT_SECONDARY_CONTRAST, WHITE, 0.82),
            )),
        36.0,
        SECONDARY_BUTTON_SURFACE,
        mix(SECONDARY_BUTTON_SURFACE, WHITE, 0.86),
        ACCENT_SECONDARY_CONTRAST,
        Some(mix(SECONDARY_BUTTON_SURFACE, BLACK, 0.91)),
        true,
        move |_, _, cx| {
            hierarchy_state.model_mut().show_hierarchy = true;
            notify(&hierarchy_entity, cx);
        },
    )
    .px(px(10.0));

    // Summary stats: `grid grid-cols-2 gap-1.5`, each `rounded p-1` on surface-1.
    let stat = |label: &'static str, value: usize| {
        div()
            .flex_1()
            .rounded(px(ROUNDED_BARE))
            .bg(rgb(SURFACE_1))
            .p(px(4.0))
            .child(ui_meta(label))
            .child(
                div()
                    .text_size(px(FONT_MD))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(TEXT_STRONG))
                    .child(value.to_string()),
            )
    };
    let stats = div()
        .flex()
        .gap(px(6.0))
        .child(stat("Total Islands", total))
        .child(stat("Active", active));

    // Show child islands.
    let merged_toggle = checkbox(
        "island-list-show-merged",
        "Show child islands",
        show_merged,
        false,
        15.0,
        true,
        {
            let state = state.clone();
            let entity = entity.clone();
            move |next, _, cx| {
                state.model_mut().show_merged = next;
                notify(&entity, cx);
            }
        },
    );

    // Search box: `Input` at `w-full !h-8 px-2 text-sm`.
    let search_box = div()
        .w_full()
        .h(px(32.0))
        .px(px(8.0))
        .rounded(px(RADIUS_SM))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .text_size(px(FONT_MD))
        .text_color(rgb(TEXT_MUTED))
        .flex()
        .items_center()
        .child(if search.is_empty() {
            "Search island ID…".to_owned()
        } else {
            search.clone()
        });

    // Sort row: ID / Volume / Layers, the active one `primary`.
    let sort_button = |id: &'static str, label: &'static str, sort: IslandSort| {
        let state = state.clone();
        let entity = entity.clone();
        let active = sort_by == sort;
        div().flex_1().child(
            button(
                id,
                label,
                if active {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Secondary
                },
                ButtonSize::Auto,
            )
            .h(px(32.0))
            .px(px(8.0))
            .w_full()
            .on_click(move |_, _, cx| {
                state.model_mut().sort_by = Some(sort);
                notify(&entity, cx);
            }),
        )
    };
    let sort_row = div()
        .flex()
        .gap(px(4.0))
        .child(sort_button("island-sort-id", "ID", IslandSort::Id))
        .child(sort_button("island-sort-volume", "Volume", IslandSort::Volume))
        .child(sort_button("island-sort-layers", "Layers", IslandSort::Layers));

    // The list: `space-y-1 max-h-[30rem] overflow-y-auto`.
    let mut list = div()
        .id("island-list-scroll")
        .flex()
        .flex_col()
        .gap(px(4.0))
        .max_h(px(480.0))
        .overflow_y_scroll();
    list = list.child(
        div()
            .px(px(4.0))
            .text_size(px(FONT_SM))
            .text_color(rgb(INDICATOR))
            .child(if rows.len() == 1 {
                "1 island".to_owned()
            } else {
                format!("{} islands", rows.len())
            }),
    );
    for row in &rows {
        list = list.child(island_list_row(
            row,
            selected_id == Some(row.id),
            layer_height_mm,
            z_offset_mm,
            {
                let state = state.clone();
                let entity = entity.clone();
                let id = row.id;
                let is_selected = selected_id == Some(row.id);
                move |_, _, cx| {
                    state.model_mut().selected_id = if is_selected { None } else { Some(id) };
                    notify(&entity, cx);
                }
            },
        ));
    }

    // Clear selection.
    let clear_state = state.clone();
    let clear_entity = entity.clone();
    let clear = div().child(
        button(
            "island-list-clear",
            "Clear Selection",
            ButtonVariant::Secondary,
            ButtonSize::Auto,
        )
        .h(px(32.0))
        .px(px(10.0))
        .w_full()
        .on_click(move |_, _, cx| {
            clear_state.model_mut().selected_id = None;
            notify(&clear_entity, cx);
        }),
    );

    // `px-2.5 pt-1 pb-2.5 space-y-1.5`.
    div()
        .px(px(10.0))
        .pt(px(4.0))
        .pb(px(10.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(div().flex().justify_end().child(hierarchy))
        .child(stats)
        .child(merged_toggle)
        .child(div().flex().flex_col().gap(px(4.0)).child(search_box).child(sort_row))
        .child(list)
        .when(selected_id.is_some(), |this| this.child(clear))
}

/// One island row: `p-1.5 rounded cursor-pointer border`, accent-tinted when
/// selected.
fn island_list_row(
    island: &IslandRow,
    selected: bool,
    layer_height_mm: f32,
    z_offset_mm: f32,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let layer_span = island.last_layer.saturating_sub(island.first_layer) + 1;
    let merged = !island.active;

    let mut badges = div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .flex_wrap()
        .mb(px(2.0))
        .child(
            div()
                .text_size(px(FONT_XS))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(format!("#{}", island.id)),
        );
    if !island.child_ids.is_empty() {
        badges = badges.child(
            div()
                .px(px(4.0))
                .py(px(2.0))
                .rounded(px(ROUNDED_BARE))
                .bg(rgba(with_alpha(GREEN_500, 0x33)))
                .text_size(px(10.0))
                .line_height(px(10.0))
                .text_color(rgb(GREEN_400))
                .child(format!("P+{}", island.child_ids.len())),
        );
    }
    if merged {
        if let Some(parent_id) = island.parent_id {
            badges = badges.child(
                div()
                    .px(px(4.0))
                    .py(px(2.0))
                    .rounded(px(ROUNDED_BARE))
                    .bg(rgba(with_alpha(ORANGE_500, 0x33)))
                    .text_size(px(10.0))
                    .line_height(px(10.0))
                    .text_color(rgb(ORANGE_400))
                    .child(format!("C#{parent_id}")),
            );
        }
    }

    let mut details = div()
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(px(FONT_SM))
                .line_height(px(FONT_SM * 1.25))
                .text_color(rgb(INDICATOR))
                .child(format!(
                    "L{}–{} ({layer_span})",
                    island.first_layer, island.last_layer
                )),
        )
        .child(
            div()
                .text_size(px(FONT_SM))
                .line_height(px(FONT_SM * 1.25))
                .text_color(rgb(INDICATOR))
                .child(match island.volume_mm3 {
                    Some(volume) => format!("{} mm³", format_number(volume)),
                    None => format!("{} mm²", format_number(island.total_area_mm2)),
                }),
        );
    if let (Some(max_area), Some(max_layer)) = (island.max_area_mm2, island.max_area_layer) {
        let layer = if layer_height_mm > 0.0 {
            (max_layer + z_offset_mm / layer_height_mm).round()
        } else {
            max_layer.round()
        };
        details = details.child(
            div()
                .text_size(px(FONT_SM))
                .line_height(px(FONT_SM * 1.25))
                .text_color(rgb(mix(TEXT_MUTED, BLACK, 0.80)))
                .child(format!("Max: {} mm² @ L{layer}", format_number(max_area))),
        );
    }

    let mut body = div()
        .flex()
        .items_start()
        .justify_between()
        .gap(px(6.0))
        .child(div().flex_1().min_w(px(0.0)).child(badges).child(details));

    if merged {
        if let Some(parent_id) = island.parent_id {
            body = body.child(
                div()
                    .mt(px(4.0))
                    .pt(px(2.0))
                    .border_t(px(1.0))
                    .border_color(rgb(BORDER_SUBTLE))
                    .text_size(px(FONT_SM))
                    .text_color(rgb(INDICATOR))
                    .child(format!("→ Child of #{parent_id}")),
            );
        }
    }
    if !island.child_ids.is_empty() {
        let children = island
            .child_ids
            .iter()
            .map(|id| format!("#{id}"))
            .collect::<Vec<_>>()
            .join(", ");
        body = body.child(
            div()
                .mt(px(4.0))
                .pt(px(2.0))
                .border_t(px(1.0))
                .border_color(rgb(BORDER_SUBTLE))
                .text_size(px(FONT_SM))
                .text_color(rgb(INDICATOR))
                .child(format!("Children: {children}")),
        );
    }

    let (background, border) = if selected {
        (
            mix(ACCENT, SURFACE_0, 0.14),
            mix(ACCENT, BORDER_SUBTLE, 0.55),
        )
    } else {
        (SURFACE_1, 0x00000000)
    };

    div()
        .id(format!("island-row-{}", island.id))
        .p(px(6.0))
        .rounded(px(ROUNDED_BARE))
        .border_1()
        .border_color(rgba(border))
        .bg(rgb(background))
        .cursor_pointer()
        .when(!selected, |this| {
            this.hover(|style| style.bg(rgba(with_alpha(BLACK, 0x1a))))
        })
        .child(body)
        .on_click(on_click)
}

use gpui::{
    Div, FontWeight, IntoElement, SharedString, WeakEntity, div, prelude::*, px, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_core::support_state::{NO_ACTIVE_PRESET_LABEL, field_help_text, *};
use gpui_core::theme::*;
use gpui_ui::panel_fields::with_alpha;
use gpui_ui::ui::{SelectOption, select_dropdown};

use super::shared::{
    accent_secondary_action_bg, accent_secondary_action_border, flat_button, labeled_number_field,
    labeled_toggle_field, normalize_value, notify,
};
use crate::DragonFruit;

// ---------------------------------------------------------------------------
// Support settings — `AutoSupportSettingsBody.tsx`
// ---------------------------------------------------------------------------
/// Duplicate / Export / Import have no store behind them.
pub fn support_settings_body(
    state: &SupportSettingsState,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let model = state.model();
    let draft = model.draft;
    let preset = model.preset;
    let modified = model.modified;
    let preset_menu_open = model.preset_menu_open;
    drop(model);
    // Every preset this shell ships is a built-in, so selecting one locks the
    // policy fields: an edit made here could never be kept.
    let locked = preset.is_some();

    // ── Preset strip ────────────────────────────────────────────────────────
    let trigger_label: SharedString = match preset {
        Some(preset) => {
            if modified {
                format!("{} *", preset.label()).into()
            } else {
                preset.label().into()
            }
        }
        None => NO_ACTIVE_PRESET_LABEL.into(),
    };
    let options: Vec<SelectOption> = [AutoSupportPreset::Light, AutoSupportPreset::Medium, AutoSupportPreset::Heavy]
        .into_iter()
        .map(|preset| {
            let mut option = SelectOption::new(preset.id(), preset.label());
            option.right = Some(format!("{} · Built-in", preset.tier()).into());
            option
        })
        .collect();
    let current_id: SharedString = match preset {
        Some(preset) => preset.id().into(),
        None => "".into(),
    };

    let menu_state = state.clone();
    let menu_entity = entity.clone();
    let toggle_entity = entity.clone();
    let toggle_state = state.clone();
    let selector = select_dropdown(
        "support-preset-select",
        &options,
        current_id.as_ref(),
        preset_menu_open,
        false,
        move |_, _, cx| {
            let open = toggle_state.model().preset_menu_open;
            toggle_state.model_mut().preset_menu_open = !open;
            notify(&toggle_entity, cx);
        },
        move |index, _, cx| {
            let preset = [AutoSupportPreset::Light, AutoSupportPreset::Medium, AutoSupportPreset::Heavy]
                .get(index)
                .copied();
            let mut model = menu_state.model_mut();
            model.preset_menu_open = false;
            if let Some(preset) = preset {
                let values = preset.values();
                model.draft = values;
                model.baseline = values;
                model.preset = Some(preset);
                model.modified = false;
            }
            drop(model);
            notify(&menu_entity, cx);
        },
    );
    // The trigger's label is the preset name; `select_dropdown` derives it from
    // the option list, so an unselected state needs its own label.
    let selector = if preset.is_none() {
        div()
            .relative()
            .w_full()
            .child(selector)
            .child(
                div()
                    .absolute()
                    .left(px(10.0))
                    .right(px(40.0))
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .flex()
                    .items_center()
                    .text_size(px(12.0))
                    .text_color(rgb(TEXT_STRONG))
                    .child(trigger_label.clone()),
            )
    } else {
        div().w_full().child(selector)
    };

    let strip_button = |id: &'static str, icon_name: &'static str, label: &'static str, enabled: bool| {
        flat_button(
            id,
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(icon(icon_name, 14.0, if enabled { TEXT_STRONG } else { TEXT_MUTED }))
                .child(label),
            32.0,
            SURFACE_1,
            BORDER_SUBTLE,
            TEXT_STRONG,
            Some(SURFACE_2),
            enabled,
            |_, _, _| {},
        )
    };
    let has_preset = preset.is_some();
    // `Duplicate` is the way out of a locked built-in: the copy is a custom
    // profile, so the policy fields unlock and Save can write it.
    let duplicate_state = state.clone();
    let duplicate_entity = entity.clone();
    let duplicate = flat_button(
        "support-preset-duplicate",
        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .child(icon(
                "copy",
                14.0,
                if has_preset { TEXT_STRONG } else { TEXT_MUTED },
            ))
            .child("Duplicate"),
        32.0,
        SURFACE_1,
        BORDER_SUBTLE,
        TEXT_STRONG,
        Some(SURFACE_2),
        has_preset,
        move |_, _, cx| {
            let mut model = duplicate_state.model_mut();
            model.preset = None;
            model.baseline = model.draft;
            model.modified = false;
            drop(model);
            notify(&duplicate_entity, cx);
        },
    );
    let strip = div()
        .rounded(px(ROUNDED_XL))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_2))
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(div().flex_1().min_w(px(0.0)).child(selector))
                .child(strip_button("support-preset-rename", "pen-line", "Rename", false))
                .child(duplicate)
                .child(strip_button("support-preset-export", "download", "Export", has_preset))
                .child(strip_button("support-preset-import", "upload", "Import", true)),
        );

    // ── Density & Sizing ────────────────────────────────────────────────────
    let density_card = field_card(
        "Density & Sizing",
        Some("How many supports, and how thick"),
        vec![
            labeled_number_field(
                "support-area-per-support",
                "Support Density",
                field_help_text(locked, "Projected area each support carries — smaller = more, tighter supports (grid spacing ≈ √value)"),
                draft.area_per_support_mm2,
                Some("mm²"),
                0.5,
                1.0,
                30.0,
                locked,
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let mut model = state.model_mut();
                        model.draft.area_per_support_mm2 = normalize_value(next, 1.0, 30.0, 0.5);
                        model.modified = true;
                        drop(model);
                        notify(&entity, cx);
                    }
                },
            ),
            labeled_number_field(
                "support-size-scale",
                "Support Size",
                field_help_text(locked, "Master multiplier over the preset sizing bands — thicker or thinner everywhere"),
                draft.size_scale,
                Some("×"),
                0.05,
                0.5,
                2.0,
                locked,
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let mut model = state.model_mut();
                        model.draft.size_scale = normalize_value(next, 0.5, 2.0, 0.05);
                        model.modified = true;
                        drop(model);
                        notify(&entity, cx);
                    }
                },
            ),
            labeled_number_field(
                "support-grid-threshold",
                "Grid Threshold",
                field_help_text(locked, "Flat regions at/above this area get a full grid; smaller regions get a single support"),
                draft.grid_area_threshold_mm2,
                Some("mm²"),
                5.0,
                5.0,
                200.0,
                locked,
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let mut model = state.model_mut();
                        model.draft.grid_area_threshold_mm2 = normalize_value(next, 5.0, 200.0, 5.0);
                        model.modified = true;
                        drop(model);
                        notify(&entity, cx);
                    }
                },
            ),
            labeled_number_field(
                "support-flat-boost",
                "Flat Boost",
                field_help_text(locked, "Grid spacing on flat ceilings — lower = denser supports on anchor surfaces (0.7 = ~2× the supports)"),
                draft.flat_density_boost,
                Some("×"),
                0.05,
                0.5,
                1.0,
                locked,
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let mut model = state.model_mut();
                        model.draft.flat_density_boost = normalize_value(next, 0.5, 1.0, 0.05);
                        model.modified = true;
                        drop(model);
                        notify(&entity, cx);
                    }
                },
            ),
            labeled_number_field(
                "support-slope-relax",
                "Slope Relax",
                field_help_text(locked, "Grid spacing on slopes at the self-support angle — higher = sparser"),
                draft.slope_relax_factor,
                Some("×"),
                0.1,
                1.0,
                2.0,
                locked,
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let mut model = state.model_mut();
                        model.draft.slope_relax_factor = normalize_value(next, 1.0, 2.0, 0.1);
                        model.modified = true;
                        drop(model);
                        notify(&entity, cx);
                    }
                },
            ),
            labeled_number_field(
                "support-suction-scale",
                "Suction Scale",
                field_help_text(locked, "How strongly flat density grows with region area — large shallow ceilings carry more peel. 0 = off"),
                draft.suction_area_exponent,
                None,
                0.05,
                0.0,
                0.4,
                locked,
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let mut model = state.model_mut();
                        model.draft.suction_area_exponent = normalize_value(next, 0.0, 0.4, 0.05);
                        model.modified = true;
                        drop(model);
                        notify(&entity, cx);
                    }
                },
            ),
        ],
    );

    // ── Stability ───────────────────────────────────────────────────────────
    let stability_card = field_card(
        "Stability",
        Some("Held against toppling and peel"),
        vec![
            labeled_number_field(
                "support-self-support-angle",
                "Self-Support Angle",
                field_help_text(locked, "Surfaces flatter than this angle get supports (resin standard: 45°). Higher = fewer, mostly on the steepest parts."),
                draft.self_support_angle_deg,
                Some("°"),
                5.0,
                20.0,
                75.0,
                locked,
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let angle = normalize_value(next, 20.0, 75.0, 5.0);
                        state.model_mut().draft.self_support_angle_deg = angle;
                        state.model_mut().modified = true;
                        // Wired: the app classifies overhangs with this value.
                        entity
                            .update(cx, |this, cx| {
                                this.support_angle = angle;
                                cx.notify();
                            })
                            .ok();
                    }
                },
            ),
            labeled_toggle_field(
                "support-stabilization",
                "Stabilization Anchors",
                field_help_text(locked, "Add contacts along the edge or corner a pose bears on, so a leaning part is held against toppling and peel (default on)"),
                draft.stabilization_enabled,
                locked,
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let mut model = state.model_mut();
                        model.draft.stabilization_enabled = next;
                        model.modified = true;
                        drop(model);
                        notify(&entity, cx);
                    }
                },
            ),
            labeled_toggle_field(
                "support-minima-reinforcement",
                "Minima Reinforcement",
                field_help_text(locked, "Ring each mesh minima with contacts on its own flank, so a section starts on a base instead of a needle (default on)"),
                draft.minima_reinforcement_enabled,
                locked,
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let mut model = state.model_mut();
                        model.draft.minima_reinforcement_enabled = next;
                        model.modified = true;
                        drop(model);
                        notify(&entity, cx);
                    }
                },
            ),
        ],
    );

    // ── Footer ──────────────────────────────────────────────────────────────
    // `Button variant="tinted-danger" size="md"`: the `.ui-button-tint-danger`
    // fill, with the trash glyph before the label.
    let delete = flat_button(
        "support-preset-delete",
        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(icon("trash-2", 14.0, DANGER))
            .child("Delete"),
        36.0,
        mix(DANGER, SURFACE_1, 0.14),
        mix(DANGER, BORDER_SUBTLE, 0.55),
        DANGER,
        Some(mix(DANGER, SURFACE_1, 0.24)),
        // `disabled={!activePreset || isBuiltIn}`: every preset here is a
        // built-in, so Delete is always greyed.
        false,
        |_, _, _| {},
    );

    let reset_state = state.clone();
    let reset_entity = entity.clone();
    let reset = flat_button(
        "support-preset-reset",
        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(icon("rotate-ccw", 14.0, TEXT_STRONG))
            .child("Reset"),
        36.0,
        SURFACE_1,
        BORDER_SUBTLE,
        TEXT_STRONG,
        Some(SURFACE_2),
        modified,
        move |_, _, cx| {
            let mut model = reset_state.model_mut();
            model.draft = model.baseline;
            model.modified = false;
            drop(model);
            notify(&reset_entity, cx);
        },
    );

    let save_state = state.clone();
    let save_entity = entity.clone();
    let save = flat_button(
        "support-preset-save",
        "Save",
        36.0,
        accent_secondary_action_bg(),
        accent_secondary_action_border(),
        ACCENT_SECONDARY,
        None,
        modified && !has_preset,
        move |_, _, cx| {
            let mut model = save_state.model_mut();
            model.baseline = model.draft;
            model.modified = false;
            drop(model);
            notify(&save_entity, cx);
        },
    );

    let footer = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .px(px(16.0))
        .py(px(12.0))
        .flex_shrink_0()
        .border_t(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgba(with_alpha(SURFACE_1, 0xe6)))
        .child(delete)
        .child(div().flex().items_center().gap(px(8.0)).child(reset).child(save));

    // `flex-1 min-h-0 flex flex-col`: the scrolling surface then the footer.
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .p(px(16.0))
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(strip)
                .child(density_card)
                .child(stability_card),
        )
        .child(footer)
}

/// A `FieldCard`: `rounded-xl border p-3` on `--surface-2`, the uppercase header,
/// its one-liner and a two-column field grid.
fn field_card(
    title: &'static str,
    subtitle: Option<&'static str>,
    fields: Vec<Div>,
) -> Div {
    let mut grid = div().flex().flex_col().gap(px(8.0));
    let mut row = div().flex().gap(px(8.0));
    let mut filled = 0;
    for field in fields {
        row = row.child(div().flex_1().min_w(px(0.0)).child(field));
        filled += 1;
        if filled == 2 {
            grid = grid.child(row);
            row = div().flex().gap(px(8.0));
            filled = 0;
        }
    }
    if filled == 1 {
        grid = grid.child(row.child(div().flex_1().min_w(px(0.0))));
    }

    let mut card = div()
        .rounded(px(ROUNDED_XL))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_2))
        .p(px(12.0))
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(px(FONT_SM))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(INDICATOR))
                .child(title.to_uppercase()),
        );
    if let Some(subtitle) = subtitle {
        card = card.child(
            div()
                .mt(px(2.0))
                .text_size(px(FONT_XS))
                .line_height(px(FONT_XS * 1.375))
                .text_color(rgb(TEXT_MUTED))
                .child(subtitle),
        );
    }
    card.child(div().mt(px(8.0)).child(grid))
}

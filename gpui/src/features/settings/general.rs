use super::*;

// ── General ──────────────────────────────────────────────────────────────

pub(crate) fn general_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let mut root = div().flex().flex_col().gap(px(12.));

    // Language
    let language_entity = entity.clone();
    root = root.child(
        section(
            "languages",
            "Language",
            "Choose the interface language. Detected from your system on first run; your choice is remembered.",
        )
        .child(
            setting_row(
                "Interface language",
                Some("Applied immediately across the app.".into()),
                SettingDensity::Compact,
                true,
                SettingSurface::Sunken,
                false,
                select_field(
                    "settings-language",
                    &LANGUAGE_OPTIONS,
                    LANGUAGE_VALUES[state.language.min(LANGUAGE_VALUES.len() - 1)],
                    Some("languages"),
                    Some(CONTROL_WIDTH),
                    true,
                    language_entity.clone(),
                    move |value, cx| {
                        if let Some(index) = LANGUAGE_VALUES.iter().position(|code| *code == value) {
                            edit_and_notify(&language_entity, cx, |draft| draft.language = index);
                        }
                    },
                ),
            )
            .mt(px(12.)),
        ),
    );

    // UI Scale
    let is_custom = state.custom_scale
        || !UI_SCALE_PRESETS
            .iter()
            .any(|preset| (*preset - state.ui_scale).abs() < 0.001);
    let scale_value = if is_custom {
        "custom"
    } else {
        UI_SCALE_OPTIONS
            .iter()
            .find(|(value, _)| {
                value
                    .parse::<f32>()
                    .map(|preset| (preset - state.ui_scale).abs() < 0.001)
                    .unwrap_or(false)
            })
            .map(|(value, _)| *value)
            .unwrap_or("custom")
    };
    let scale_entity = entity.clone();
    let mut scale_card = inner_card().child(setting_row(
        "Interface scale",
        Some("Larger percentages magnify the whole UI.".into()),
        SettingDensity::Compact,
        false,
        SettingSurface::Sunken,
        false,
        select_field(
            "settings-ui-scale",
            &UI_SCALE_OPTIONS,
            scale_value,
            Some("zoom-in"),
            Some(CONTROL_WIDTH),
            true,
            scale_entity.clone(),
            move |value, cx| {
                if value == "custom" {
                    // Selecting "Custom" arms the field but saves nothing.
                    edit_and_notify(&scale_entity, cx, |draft| draft.custom_scale = true);
                    return;
                }
                if let Ok(next) = value.parse::<f32>() {
                    edit_and_notify(&scale_entity, cx, |draft| {
                        draft.custom_scale = false;
                        draft.ui_scale = next;
                    });
                }
            },
        ),
    ));
    if is_custom {
        let custom_entity = entity.clone();
        scale_card = scale_card.child(
            div()
                .mt(px(10.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Custom scale"),
                )
                .child(number_control(
                    "settings-custom-scale",
                    state.ui_scale * 100.0,
                    "%",
                    1.0,
                    MIN_UI_SCALE * 100.0,
                    MAX_UI_SCALE * 100.0,
                    CONTROL_WIDTH,
                    move |next, cx| {
                        edit_and_notify(&custom_entity, cx, |draft| {
                            draft.ui_scale = (next / 100.0).clamp(MIN_UI_SCALE, MAX_UI_SCALE);
                        });
                    },
                )),
        );
    }
    root = root.child(
        section(
            "zoom-in",
            "UI Scale",
            "Adjusts the size of the entire interface. 100% adapts to your screen automatically.",
        )
        .child(scale_card.mt(px(12.))),
    );

    // Toolbar
    let toolbar_entity = entity.clone();
    root = root.child(
        section(
            "panel-top",
            "Toolbar",
            "Park the tools in a column on the left edge, or in a bar centred under the app bar.",
        )
        .child(
            setting_row(
                "Toolbar position",
                Some("A left column or a bar centred under the app bar.".into()),
                SettingDensity::Compact,
                true,
                SettingSurface::Sunken,
                false,
                select_field(
                    "settings-tool-layout",
                    &TOOL_LAYOUT_OPTIONS,
                    match state.tool_layout {
                        ToolLayout::Top => "horizontal",
                        ToolLayout::Left => "vertical",
                    },
                    Some(match state.tool_layout {
                        ToolLayout::Top => "arrow-up",
                        ToolLayout::Left => "arrow-left",
                    }),
                    Some(CONTROL_WIDTH),
                    true,
                    toolbar_entity.clone(),
                    move |value, cx| {
                        let next = if value == "vertical" {
                            ToolLayout::Left
                        } else {
                            ToolLayout::Top
                        };
                        edit_and_notify(&toolbar_entity, cx, |draft| draft.tool_layout = next);
                    },
                ),
            )
            .mt(px(12.)),
        ),
    );

    // Floating Windows
    let persistence_entity = entity.clone();
    let reset_entity = entity.clone();
    root = root.child(
        section(
            "layout-grid",
            "Floating Windows",
            "Keep moved panel positions between sessions, or always start from the default workspace layout.",
        )
        .child(
            setting_row(
                "Remember window positions",
                Some("Persist dragged panel positions in local storage.".into()),
                SettingDensity::Compact,
                true,
                SettingSurface::Sunken,
                false,
                segmented(
                    "settings-floating-persistence",
                    &["OFF", "ON"],
                    usize::from(state.floating_persistence),
                    false,
                    move |index, cx| {
                        edit(|draft| draft.floating_persistence = index == 1);
                        // `setFloatingLayoutPersistenceEnabled` — the panel
                        // layout owns the flag that gates writing
                        // `floating-panel-layout.json`, so the switch has to
                        // reach it as well as the saved setting.
                        persistence_entity
                            .update(cx, |this, cx| {
                                this.layout.set_persistence_enabled(index == 1);
                                cx.notify();
                            })
                            .ok();
                    },
                ),
            )
            .mt(px(12.)),
        )
        .child(
            setting_row(
                "Reset saved window layout",
                Some("Forget all stored panel positions and return to seeded layout.".into()),
                SettingDensity::Compact,
                true,
                SettingSurface::Sunken,
                false,
                button_with(
                    "settings-reset-layout",
                    "Reset",
                    ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Lg)
                        .padding_x(12.0)
                        .padding_y(0.0, 0.0)
                        .font_size(14.0)
                        .gap(6.0)
                        .icon("rotate-ccw", 16.0, TEXT_STRONG),
                )
                .on_click(move |_, _, cx| {
                    // `clearSavedFloatingLayout()` (`SettingsModal.tsx:928-960`
                    // → `floatingLayoutPreferences.ts`): forget every stored
                    // placement and delete the saved document. It is immediate
                    // and does not touch the persistence flag.
                    reset_entity
                        .update(cx, |this, cx| {
                            this.layout.reset();
                            cx.notify();
                        })
                        .ok();
                }),
            )
            .mt(px(8.)),
        ),
    );

    // Support Tooltips
    let support_entity = entity.clone();
    root = root.child(
        section(
            "circle-question-mark",
            "Support Tooltips",
            "Show placement tooltips next to the cursor while adding supports.",
        )
        .child(
            setting_row(
                "Show support tooltips",
                Some("Shows \"Cannot Place Support\" and \"Stability Warning\" tooltips.".into()),
                SettingDensity::Compact,
                true,
                SettingSurface::Sunken,
                false,
                segmented(
                    "settings-support-help",
                    &["OFF", "ON"],
                    usize::from(state.support_help),
                    false,
                    move |index, cx| {
                        edit_and_notify(&support_entity, cx, |draft| {
                            draft.support_help = index == 1;
                        });
                    },
                ),
            )
            .mt(px(12.)),
        ),
    );

    // Import Defaults
    let raft_entity = entity.clone();
    let wall_entity = entity.clone();
    let roots_entity = entity.clone();
    let repair_entity = entity.clone();
    let repair_scenes_entity = entity.clone();
    let mut import_section = section(
        "database",
        "Import Defaults",
        "Applied automatically when importing Scene Files.",
    )
    .child(
        setting_row(
            "Default Raft Base",
            Some("Chooses raft bottom mode for imported supports.".into()),
            SettingDensity::Compact,
            true,
            SettingSurface::Sunken,
            false,
            select_field(
                "settings-raft-bottom",
                &RAFT_OPTIONS,
                match state.raft_bottom {
                    RaftBottom::Off => "off",
                    RaftBottom::Line => "line",
                    RaftBottom::Solid => "solid",
                },
                None,
                Some(CONTROL_WIDTH),
                true,
                raft_entity.clone(),
                move |value, cx| {
                    let next = match value {
                        "off" => RaftBottom::Off,
                        "line" => RaftBottom::Line,
                        _ => RaftBottom::Solid,
                    };
                    edit_and_notify(&raft_entity, cx, |draft| {
                        draft.raft_bottom = next;
                        if next == RaftBottom::Line {
                            draft.roots_enabled = true;
                        }
                    });
                },
            ),
        )
        .mt(px(12.)),
    );
    if state.raft_bottom == RaftBottom::Solid {
        import_section = import_section.child(
            setting_row(
                "Default Raft Wall",
                Some("Enable perimeter wall for imported solid rafts.".into()),
                SettingDensity::Compact,
                true,
                SettingSurface::Sunken,
                false,
                segmented(
                    "settings-raft-wall",
                    &["OFF", "ON"],
                    usize::from(state.raft_wall),
                    false,
                    move |index, cx| {
                        edit_and_notify(&wall_entity, cx, |draft| {
                            draft.raft_wall = index == 1;
                        });
                    },
                ),
            )
            .mt(px(8.)),
        );
    }
    if state.raft_bottom != RaftBottom::Line {
        import_section = import_section.child(
            setting_row(
                "Roots Enabled on Import",
                Some("OFF makes imported root diameter match trunk diameter.".into()),
                SettingDensity::Compact,
                true,
                SettingSurface::Sunken,
                false,
                segmented(
                    "settings-roots",
                    &["OFF", "ON"],
                    usize::from(state.roots_enabled),
                    false,
                    move |index, cx| {
                        edit_and_notify(&roots_entity, cx, |draft| {
                            draft.roots_enabled = index == 1;
                        });
                    },
                ),
            )
            .mt(px(8.)),
        );
    }
    import_section = import_section
        .child(
            setting_row(
                "Auto-Repair",
                Some("Automatically runs native mesh auto-repair for standard mesh imports.".into()),
                SettingDensity::Compact,
                true,
                SettingSurface::Sunken,
                false,
                segmented(
                    "settings-auto-repair",
                    &["OFF", "ON"],
                    usize::from(state.auto_repair),
                    false,
                    move |index, cx| {
                        edit_and_notify(&repair_entity, cx, |draft| {
                            draft.auto_repair = index == 1;
                        });
                    },
                ),
            )
            .mt(px(8.)),
        )
        .child(
            setting_row(
                "Auto-Repair Scenes",
                Some("Automatically runs native mesh auto-repair for scene-file imports.".into()),
                SettingDensity::Compact,
                true,
                SettingSurface::Sunken,
                false,
                segmented(
                    "settings-auto-repair-scenes",
                    &["OFF", "ON"],
                    usize::from(state.auto_repair_scenes),
                    false,
                    move |index, cx| {
                        edit_and_notify(&repair_scenes_entity, cx, |draft| {
                            draft.auto_repair_scenes = index == 1;
                        });
                    },
                ),
            )
            .mt(px(8.)),
        );
    root.child(import_section)
}

use super::*;
use gpui_ui::ui::button_with_content;

// ── UVTools (`UvToolsSettingsTab.tsx`) ───────────────────────────────────

pub(crate) fn uvtools_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let uvtools = &state.uvtools;
    let path_entity = entity.clone();
    let enabled_entity = entity;
    // `detectPlatform() === 'mac'` decides the executable label.
    let executable_label = if cfg!(target_os = "macos") {
        "UVtools.app"
    } else {
        "UVTools.exe"
    };

    let mut card = section(
        "external-link",
        "UVTools Integration",
        "Automatically open sliced print files in UVTools for further inspection and repair. After slicing, the file is sent directly to UVTools for analysis.",
    );
    card = card.child(
        setting_row(
            "Enable UVTools Integration",
            Some("Adds a “Send to UVTools” option in the slicing panel.".into()),
            SettingDensity::Compact,
            true,
            SettingSurface::Sunken,
            false,
            segmented(
                "uvtools-enabled",
                &["OFF", "ON"],
                usize::from(uvtools.enabled),
                false,
                move |index, cx| {
                    edit_and_notify(&enabled_entity, cx, |draft| {
                        draft.uvtools.enabled = index == 1;
                    });
                },
            ),
        )
        .mt(px(12.)),
    );

    if uvtools.enabled {
        let (icon_slug, icon_color, label) = if uvtools.show_found_glow {
            (
                "circle-check",
                ACCENT_SECONDARY,
                "Found!",
            )
        } else if uvtools.show_not_found {
            (
                "search",
                TEXT_MUTED,
                "Auto-Discover",
            )
        } else {
            ("search", TEXT_MUTED, "Auto-Discover")
        };

        // `autoDiscoverUvToolsPath` needs the native bridge, so the button is
        // inert; `discoveryBusy` still drives the source's spinner.
        let mut discover = button_with_content(
            "uvtools-discover",
            &ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Md)
                .padding_x(12.0)
                .padding_y(0.0, 0.0)
                .font_size(14.0),
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(
                    if uvtools.discovery_busy {
                        spinner(16.).into_any_element()
                    } else {
                        icon(icon_slug, 16., icon_color).into_any_element()
                    },
                )
                .child(if uvtools.discovery_busy {
                    "Scanning…".to_string()
                } else {
                    label.to_string()
                }),
        );
        if uvtools.show_found_glow {
            // `color-mix(in srgb, accent-secondary, surface-1 88%)`.
            discover = discover.bg(rgb(mix(ACCENT_SECONDARY, SURFACE_1, 0.12)));
        } else if uvtools.show_not_found {
            // `color-mix(in srgb, danger, surface-1 90%)`.
            discover = discover.bg(rgb(mix(DANGER, SURFACE_1, 0.10)));
        }

        card = card.child(
            inner_card()
                .mt(px(8.))
                .child(setting_row(
                    "UVTools Executable Path",
                    Some(format!("Use auto-discover or enter the path to {executable_label} manually.").into()),
                    SettingDensity::Compact,
                    false,
                    SettingSurface::Sunken,
                    false,
                    discover,
                ))
                .child(
                    div().mt(px(8.)).child(text_field(
                        "uvtools-custom-path",
                        &uvtools.custom_path,
                        format!("Select or type the path to {executable_label}"),
                        true,
                        30.0,
                        12.0,
                        {
                            let path_entity = path_entity.clone();
                            move |text, _, cx| {
                                edit_and_notify(&path_entity, cx, |draft| {
                                    draft.uvtools.custom_path = text.to_owned();
                                });
                            }
                        },
                    )),
                ),
        );
    }

    div().flex().flex_col().gap(px(12.)).child(card)
}

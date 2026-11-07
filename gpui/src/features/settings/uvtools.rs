use super::*;

// ── UVTools (`UvToolsSettingsTab.tsx`) ───────────────────────────────────

pub(crate) fn uvtools_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let uvtools = &state.uvtools;
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
            "Adds a “Send to UVTools” option in the slicing panel.",
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
        let (background, icon_slug, icon_color, label) = if uvtools.show_found_glow {
            (
                mix(ACCENT_SECONDARY, SURFACE_1, 0.88),
                "circle-check",
                ACCENT_SECONDARY,
                "Found!",
            )
        } else if uvtools.show_not_found {
            (
                mix(DANGER, SURFACE_1, 0.90),
                "search",
                TEXT_MUTED,
                "Auto-Discover",
            )
        } else {
            (SURFACE_1, "search", TEXT_MUTED, "Auto-Discover")
        };

        // `autoDiscoverUvToolsPath` needs the native bridge, so the button is
        // inert; `discoveryBusy` still drives the source's spinner.
        let mut discover = div()
            .h(px(36.))
            .px(px(12.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(6.))
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(BORDER_SUBTLE))
            .bg(rgb(background))
            .text_size(px(14.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(TEXT_STRONG));
        if uvtools.discovery_busy {
            discover = discover.child(spinner(16.));
        } else {
            discover = discover.child(icon(icon_slug, 16., icon_color));
        }
        discover = discover.child(if uvtools.discovery_busy {
            "Scanning…"
        } else {
            label
        });

        card = card.child(
            inner_card()
                .mt(px(8.))
                .child(plain_row(
                    "UVTools Executable Path",
                    format!("Use auto-discover or enter the path to {executable_label} manually."),
                    discover,
                ))
                .child(
                    div().mt(px(8.)).child(text_field(
                        &uvtools.custom_path,
                        &format!("Select or type the path to {executable_label}"),
                        true,
                        30.0,
                        12.0,
                    )),
                ),
        );
    }

    div().flex().flex_col().gap(px(12.)).child(card)
}

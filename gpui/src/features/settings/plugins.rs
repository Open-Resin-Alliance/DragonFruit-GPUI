use super::*;

// ── Plugins (`PluginsSettingsTab.tsx`) ───────────────────────────────────

pub(crate) fn plugins_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let plugins = &state.plugins;
    let allowance_entity = entity.clone();
    let install_entity = entity.clone();
    let repo_entity = entity.clone();

    // `installedRows` is empty: the profile store and the generated built-in
    // plugin manifests are not part of this slice.
    let installed_count = 0usize;

    let mut root = div().flex().flex_col().gap(px(12.));

    let mut header = section(
        "plug",
        "Plugins",
        "Profile packs add presets and material profiles. A sideloaded package adds code and needs a publisher you trust.",
    )
    .relative();
    // `IconButton` absolute right-3 top-3, `PackageOpen`, warning tone.
    header = header.child(
        icon_button_toned(
            "plugins-allowance",
            "package-open",
            16.0,
            IconButtonVariant::Surface,
            IconButtonSize::Md,
            IconTone::Warning,
            false,
        )
        .absolute()
        .top(px(12.))
        .right(px(12.))
        .on_click(move |_, _, cx| {
            edit_and_notify(&allowance_entity, cx, |draft| {
                draft.plugins.allowance_dialog_open = true;
            });
        }),
    );

    let install_row = setting_row(
        "Install profile pack",
        Some("Adds presets and materials from a repository. Contains no code.".into()),
        SettingDensity::Compact,
        true,
        SettingSurface::Sunken,
        false,
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .w(px(420.))
            .child(
                div().flex_1().min_w(px(0.)).child(text_field(
                    "plugins-repo-url",
                    &plugins.repo_url,
                    "https://github.com/owner/repo",
                    false,
                    34.0,
                    14.0,
                    {
                        let repo_entity = repo_entity.clone();
                        move |text, _, cx| {
                            edit_and_notify(&repo_entity, cx, |draft| {
                                draft.plugins.repo_url = text.to_owned();
                            });
                        }
                    },
                )),
            )
            .child(
                button_with(
                    "plugins-install",
                    "Install",
                    ButtonSpec::new(ButtonVariant::TintedAccent, ButtonSize::Md)
                        .padding_x(12.0)
                        .padding_y(0.0, 0.0)
                        .font_size(12.0)
                        .gap(6.0)
                        .icon("download", 14.0, ACCENT),
                ),
            ),
    );

    let mut rows = div().mt(px(8.)).flex().flex_col().gap(px(8.)).child(install_row);
    if plugins.allow_sideload {
        rows = rows.child(setting_row(
            "Import package",
            Some("Checks the publisher signature, then unseals and installs the .dfplugin file.".into()),
            SettingDensity::Compact,
            true,
            SettingSurface::Sunken,
            false,
            button_with(
                "plugins-import",
                "Choose file…",
                ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Md)
                    .padding_x(12.0)
                    .padding_y(0.0, 0.0)
                    .font_size(12.0)
                    .gap(6.0)
                    .icon("package-plus", 14.0, TEXT_STRONG),
            ),
        ));
    }
    header = header.child(rows);
    if let Some((error, message)) = &plugins.status {
        header = header.child(
            div()
                .mt(px(8.))
                .text_size(px(12.))
                .text_color(rgb(if *error { 0xfca5a5 } else { 0x86efac }))
                .child(message.clone()),
        );
    }
    root = root.child(header);

    // Installed plugins
    let mut installed = section(
        "package-check",
        "Installed plugins",
        if installed_count == 1 {
            "1 plugin in this profile."
        } else {
            "0 plugins in this profile."
        },
    );
    installed = installed.child(
        div()
            .mt(px(8.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .px(px(12.))
                    .py(px(8.))
                    .text_size(px(12.))
                    .text_color(rgb(TEXT_MUTED))
                    .child("No plugins installed in this profile."),
            ),
    );
    root = root.child(installed);

    // `isDevRuntime` is false for a release build, so the Plugin Creation Studio
    // link is absent, exactly as at the source.
    let _ = install_entity;
    root
}

/// The `Sideloaded packages` allowance dialog.
pub(crate) fn plugins_overlay(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Option<Div> {
    if !state.plugins.allowance_dialog_open {
        return None;
    }
    let plugins = &state.plugins;
    let cancel_entity = entity.clone();
    let toggle_entity = entity.clone();

    let body = if plugins.allow_sideload {
        div().child(
            div()
                .text_size(px(14.))
                .line_height(relative(1.45))
                .text_color(rgb(TEXT_MUTED))
                .child("Packages you import run with your permissions and can read any file you can read, open network connections, and crash the app if they misbehave. Disabling this stops new imports; packages already installed stay on disk."),
        )
    } else {
        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(relative(1.45))
                    .text_color(rgb(TEXT_STRONG))
                    .child("A sideloaded package is a native program, not a profile pack."),
            )
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(relative(1.45))
                    .text_color(rgb(TEXT_MUTED))
                    .child("Once imported it runs with your permissions: it can read any file you can read, open network connections, and crash the app if it misbehaves."),
            )
    };

    let close_entity = entity;
    Some(dialog_modal_full(
        512.0, // max-w-lg
        if plugins.allow_sideload {
            "Sideloading is on"
        } else {
            "Allow untrusted plugins?"
        },
        Some("Sideloaded packages".into()),
        Some("shield-alert"),
        IconTone::Warning,
        false,
        body.child(
            div()
                .mt(px(12.))
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(mix(0xd97706, BORDER_SUBTLE, 0.4)))
                .bg(rgb(mix(0xd97706, SURFACE_1, 0.92)))
                .px(px(12.))
                .py(px(8.))
                .flex()
                .items_start()
                .gap(px(8.))
                .child(icon("triangle-alert", 14., 0xd97706))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(12.))
                        .line_height(relative(1.45))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Signatures prove who built the package, not that it is safe. Only enable this for a publisher you trust."),
                ),
        ),
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(
                button_with(
                    "plugins-allowance-cancel",
                    "Cancel",
                    ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Lg)
                        .padding_x(16.0)
                        .padding_y(0.0, 0.0)
                        .font_size(14.0),
                )
                .on_click(move |_, _, cx| {
                    edit_and_notify(&cancel_entity, cx, |draft| {
                        draft.plugins.allowance_dialog_open = false;
                    });
                }),
            )
            .child(
                button_with(
                    "plugins-allowance-toggle",
                    if plugins.allow_sideload {
                        "Disable Sideloading"
                    } else {
                        "Enable Sideloading"
                    },
                    ButtonSpec::new(
                        if plugins.allow_sideload {
                            ButtonVariant::TintedDanger
                        } else {
                            ButtonVariant::TintedWarning
                        },
                        ButtonSize::Lg,
                    )
                    .padding_x(16.0)
                    .padding_y(0.0, 0.0)
                    .font_size(14.0)
                    .gap(6.0)
                    .icon(
                        "shield-alert",
                        14.0,
                        if plugins.allow_sideload {
                            DANGER
                        } else {
                            0xf59e0b
                        },
                    ),
                )
                .on_click(move |_, _, cx| {
                    edit_and_notify(&toggle_entity, cx, |draft| {
                        draft.plugins.allow_sideload = !draft.plugins.allow_sideload;
                        draft.plugins.allowance_dialog_open = false;
                    });
                }),
            ),
        move |_, _, cx| {
            edit_and_notify(&close_entity, cx, |draft| {
                draft.plugins.allowance_dialog_open = false;
            });
        },
    ))
}

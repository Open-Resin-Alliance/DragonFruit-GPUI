//! The `Select` trigger: option types and the dropdown entry point.

use gpui::{App, ClickEvent, Div, SharedString, Stateful, Window};

use super::select_menu::select_dropdown_aligned;

// ── Select / SelectDropdown ─────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SelectOptionTone {
    Default,
    Accent,
    Danger,
}

/// One entry of a [`select_dropdown`] menu.
pub struct SelectOption {
    pub value: SharedString,
    pub label: SharedString,
    pub disabled: bool,
    /// Render as a non-interactive group header rather than a choice.
    pub heading: bool,
    /// Render as a thin rule, for separating a destructive choice.
    pub divider: bool,
    /// A command rather than a value: it never shows as the field's choice.
    pub action: bool,
    /// Right-aligned secondary content (`text-[10px]`, `--text-muted`).
    pub right: Option<SharedString>,
    pub tone: SelectOptionTone,
    pub icon: Option<SharedString>,
}

impl SelectOption {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
            heading: false,
            divider: false,
            action: false,
            right: None,
            tone: SelectOptionTone::Default,
            icon: None,
        }
    }

    pub fn heading(label: impl Into<SharedString>) -> Self {
        let label = label.into();
        Self {
            value: label.clone(),
            label,
            disabled: false,
            heading: true,
            divider: false,
            action: false,
            right: None,
            tone: SelectOptionTone::Default,
            icon: None,
        }
    }

    pub fn divider() -> Self {
        Self {
            value: SharedString::default(),
            label: SharedString::default(),
            disabled: false,
            heading: false,
            divider: true,
            action: false,
            right: None,
            tone: SelectOptionTone::Default,
            icon: None,
        }
    }

    /// A command row: it runs `on_select` but never reads as the field's value.
    pub fn action(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            action: true,
            ..Self::new(value, label)
        }
    }

    pub fn with_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn with_right(mut self, right: impl Into<SharedString>) -> Self {
        self.right = Some(right.into());
        self
    }

    pub fn with_tone(mut self, tone: SelectOptionTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

/// `menuAlign` — which edge of the trigger the menu is pinned to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SelectMenuAlign {
    Left,
    Right,
}

/// `Select.tsx` + `SelectDropdown.tsx` — the trigger and its menu.
///
/// The trigger is `.ui-input rounded-[4px] relative w-full h-[36px] px-2.5 py-0
/// pr-10 leading-none text-sm inline-flex items-center text-left`; the open
/// state swaps the border to `color-mix(accent, border-subtle 18%)` and adds the
/// `0 0 0 1px color-mix(accent, transparent 72%) inset` ring. The menu is
/// `rounded-[4px] border shadow-xl` on `color-mix(surface-0, black 6%)`.
///
/// React portals the menu to `<body>`; GPUI has no portal, so it renders as an
/// absolute child under the trigger, 6px below it and matching its width. The
/// source clamps the menu to the space below the trigger; with no measure pass
/// here, the list is capped at 320px and scrolls. The chevron flips from
/// `chevron-down` to `chevron-up` in place of `rotate-180`.
///
/// Not ported, because each needs DOM measurement or an overlay the port has no
/// equivalent for: `endAdornment` (`absolute right-8`), the overlay display
/// (`selectedDisplay` / `hideSelectedText` / `selectedTextAlign` /
/// `selectedDisplayOffsetX`), `hideChevron`/`chevronClassName`, and
/// `menuFooterAction`. `menuAlign` is [`select_dropdown_aligned`].
///
/// `on_select` receives the index of the chosen option.
pub fn select_dropdown(
    id: impl Into<SharedString>,
    options: &[SelectOption],
    value: &str,
    open: bool,
    disabled: bool,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select: impl Fn(usize, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    select_dropdown_aligned(
        id,
        options,
        value,
        open,
        disabled,
        SelectMenuAlign::Left,
        on_toggle,
        on_select,
    )
}

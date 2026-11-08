//! Editable `.ui-input` fields for the panes that render without an `App`.
//!
//! The settings panes, the printer-profile editor and the printer picker are
//! free functions re-run every frame with a snapshot of their state and a
//! `WeakEntity<DragonFruit>`; none of them receives an `App`, and the element
//! that GPUI draws is built before the frame starts. [`gpui_ui::ui::text_field`]
//! and [`gpui_ui::ui::number_field`] need the [`TextInput`] entity they drive
//! at that point, which only an `App` can create.
//!
//! [`EditableField`] closes that gap the way `windows(..)` closes it for any
//! other deferred work: it is a passthrough element that builds its child in
//! `request_layout`, the first callback in the frame that receives both
//! `Window` and `App`. The child is laid out, prepainted and painted exactly as
//! `div`'s own children are, so the frame cost is one extra passthrough node,
//! not a re-implementation of the field.
//!
//! The entity is cached per field id in [`INPUTS`], so focus, selection and the
//! caret survive frames. Each frame mirrors the caller's value into the text
//! while the field is unfocused — the rule `LabeledInput`, `NumberInput` and
//! `ScrollableNumberField` all share (`src/components/settings/
//! profileFormAtoms.tsx:190-193`, `src/components/ui/NumberInput.tsx:63-69`).
//! While the field is focused the text is the user's and is never rewritten.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Element, ElementId, Entity, FocusHandle, GlobalElementId, IntoElement,
    LayoutId, SharedString, Window, div, prelude::*, px, rgb,
};
use gpui_locale::current_catalog;
use gpui_ui::text_input::TextInput;
use gpui_ui::ui::{input_field, parse_number, text_field};

/// One entity per field id. `Entity` is not `Send`, so this is thread-local.
thread_local! {
    static INPUTS: RefCell<HashMap<SharedString, Entity<TextInput>>> =
        RefCell::new(HashMap::new());
}

/// The entity behind `key`, created with `initial` on first layout.
fn entity_for(key: &SharedString, initial: &str, cx: &mut App) -> Entity<TextInput> {
    INPUTS.with(|inputs| {
        if let Some(entity) = inputs.borrow().get(key) {
            return entity.clone();
        }
        let entity = cx.new(|cx| TextInput::new(cx, initial.to_owned()));
        inputs.borrow_mut().insert(key.clone(), entity.clone());
        entity
    })
}

/// The active UI locale, as `gpui_locale::set_locale` left it for this thread.
fn locale() -> gpui_locale::Locale {
    current_catalog().locale().clone()
}

// ── The passthrough element ─────────────────────────────────────────────────

type BuildFn = Rc<RefCell<dyn FnMut(&mut Window, &mut App) -> AnyElement>>;

/// An element that builds its child once per frame, where `App` is available.
///
/// Its layout, prepaint and paint delegate to the child exactly as
/// [`gpui::AnyElement`]'s own `Element` impl does, so the child's taffy node is
/// the node the parent lays out.
pub(crate) struct EditableField {
    build: BuildFn,
}

impl EditableField {
    fn new(build: impl FnMut(&mut Window, &mut App) -> AnyElement + 'static) -> Self {
        Self {
            build: Rc::new(RefCell::new(build)),
        }
    }
}

impl Element for EditableField {
    type RequestLayoutState = AnyElement;
    type PrepaintState = Option<FocusHandle>;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut child = (self.build.borrow_mut())(window, cx);
        let layout_id = child.request_layout(window, cx);
        (layout_id, child)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _bounds: gpui::Bounds<gpui::Pixels>,
        child: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        child.prepaint(window, cx)
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _bounds: gpui::Bounds<gpui::Pixels>,
        child: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        child.paint(window, cx);
    }
}

impl IntoElement for EditableField {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

// ── Fields ──────────────────────────────────────────────────────────────────

/// `.ui-input` for free text, committed **on every keystroke**.
///
/// This is the source's `LabeledInput` (`profileFormAtoms.tsx:201-212`) and the
/// theme hex `<input>` (`UISettingsTab.tsx:246-254`): both call `onChange` with
/// each edit, and keep the typed text while focused.
///
/// `disabled` is the source's `disabled` + `opacity-55 cursor-not-allowed`
/// state: the box paints the value and takes no input.
pub(crate) fn text_input(
    key: impl Into<SharedString>,
    value: impl Into<SharedString>,
    placeholder: impl Into<SharedString>,
    disabled: bool,
    height: f32,
    on_change: impl Fn(&str, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let key: SharedString = key.into();
    let value: SharedString = value.into();
    let placeholder: SharedString = placeholder.into();
    if disabled {
        return input_field(ElementId::Name(key), value, false)
            .h(px(height))
            .opacity(0.55)
            .cursor_not_allowed()
            .into_any_element();
    }

    let on_change = Rc::new(on_change);
    EditableField::new(move |window, cx| {
        let entity = entity_for(&key, &value, cx);
        entity.update(cx, |field, _| {
            if !field.is_focused(window) {
                field.set_display_value(value.to_string());
            }
        });
        let on_change = on_change.clone();
        let placeholder = placeholder.clone();
        text_field(&entity, placeholder, move |text, window, cx| {
            on_change(text, window, cx)
        })
        .h(px(height))
        .into_any_element()
    })
    .into_any_element()
}

/// `fmt.field(value, max_digits)` in the active locale, without grouping.
///
/// The source's number fields show the value with at most `max_digits` fraction
/// digits and no trailing zeros (`String(value)` for `LabeledNumberInput`,
/// `fmt.field(value, 2)` for `NumberInput`), unlike [`format_number`]'s fixed
/// digits. Group separators are stripped because the text is parsed back.
fn field_text(value: f32, max_digits: usize) -> SharedString {
    let locale = locale();
    let text = gpui_locale::fmt::field(&locale, value as f64, max_digits);
    let group = locale.group_separator();
    if group.is_empty() || group == locale.decimal_separator() {
        text
    } else {
        text.to_string().replace(group, "").into()
    }
}

/// A numeric `.ui-input` that commits **as the user types**.
///
/// This is the source's `NumberInput` (`NumberInput.tsx:74-97`): every keystroke
/// that parses writes the value, non-numbers are ignored, and the text is only
/// reformatted once the field loses focus.
///
/// `min`/`max` are stored units; `precision` is the number of fraction digits
/// the committed value is rounded to.
pub(crate) fn number_typed(
    key: impl Into<SharedString>,
    value: f32,
    min: f32,
    max: f32,
    precision: usize,
    disabled: bool,
    height: f32,
    on_change: impl Fn(f32, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let key: SharedString = key.into();
    if disabled {
        return input_field(ElementId::Name(key), field_text(value, precision), false)
            .h(px(height))
            .opacity(0.55)
            .cursor_not_allowed()
            .into_any_element();
    }

    let on_change = Rc::new(on_change);
    EditableField::new(move |window, cx| {
        let locale = locale();
        let display = field_text(value, precision);
        let entity = entity_for(&key, &display, cx);
        entity.update(cx, |field, _| {
            if !field.is_focused(window) {
                field.set_display_value(display.to_string());
            }
        });
        let on_change = on_change.clone();
        let min = if min.is_finite() { Some(min) } else { None };
        let max = if max.is_finite() { Some(max) } else { None };
        text_field(&entity, "", move |text, window, cx| {
            let next = parse_number(&locale, text, value, min, max, precision);
            if next != value {
                on_change(next, window, cx);
            }
        })
        .h(px(height))
        .into_any_element()
    })
    .into_any_element()
}

/// A numeric `.ui-input` that commits **on blur** (and on Enter, which the
/// source treats as a blur).
///
/// The source's `LabeledNumberInput` keeps a local string while focused and
/// calls its `commit()` from `onBlur` and Enter
/// (`profileFormAtoms.tsx:329-345`, `:369-386`); `ScrollableNumberField`'s
/// `commitOnBlur` mode does the same. Nothing is written while typing, a value
/// that does not parse restores the last committed one, and the field shows the
/// canonical form again once it is committed.
///
/// `unit` is `Some((suffix, scale))` for a field that shows a different unit
/// than it stores (`Some(("µm", 1000.0))`); the callback always reports the
/// stored unit.
pub(crate) fn number_on_blur(
    key: impl Into<SharedString>,
    value: f32,
    unit: Option<(&'static str, f32)>,
    min: f32,
    max: f32,
    precision: usize,
    disabled: bool,
    height: f32,
    on_change: impl Fn(f32, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let key: SharedString = key.into();
    let scale = unit.map(|(_, scale)| scale).unwrap_or(1.0);
    if disabled {
        let mut box_ = input_field(
            ElementId::Name(key),
            field_text(value * scale, precision),
            false,
        )
        .h(px(height))
        .opacity(0.55)
        .cursor_not_allowed();
        if let Some((suffix, _)) = unit {
            box_ = box_.child(
                div()
                    .ml(px(4.0))
                    .text_size(px(gpui_core::theme::FONT_XS))
                    .text_color(rgb(gpui_core::theme::TEXT_MUTED))
                    .child(suffix),
            );
        }
        return box_.into_any_element();
    }

    let on_change = Rc::new(on_change);
    EditableField::new(move |window, cx| {
        let locale = locale();
        let display = field_text(value * scale, precision);
        let entity = entity_for(&key, &display, cx);

        let (focused, transition) = entity.update(cx, |field, _| {
            (field.is_focused(window), field.note_focus(window))
        });
        if !focused {
            if transition == Some(false) {
                // Blur: commit whatever the field holds, then show the
                // canonical form (which is also the "restore" path when the
                // text does not parse).
                let text = entity.read(cx).value().to_owned();
                let next = parse_number(
                    &locale,
                    &text,
                    value * scale,
                    Some(min * scale),
                    Some(max * scale),
                    precision,
                ) / scale;
                let canonical = field_text(next * scale, precision).to_string();
                entity.update(cx, |field, _| field.set_display_value(canonical));
                if next != value {
                    on_change(next, window, cx);
                }
            } else {
                entity.update(cx, |field, _| field.set_display_value(display.to_string()));
            }
        }

        let on_change = on_change.clone();
        let submit_entity = entity.clone();
        let submit = move |text: &str, window: &mut Window, cx: &mut App| {
            let next = parse_number(
                &locale,
                text,
                value * scale,
                Some(min * scale),
                Some(max * scale),
                precision,
            ) / scale;
            let canonical = field_text(next * scale, precision).to_string();
            submit_entity.update(cx, |field, _| field.set_display_value(canonical));
            if next != value {
                on_change(next, window, cx);
            }
        };
        text_field(&entity, "", move |text, window, cx| {
            // The source commits nothing while typing; the text is the field's.
            let _ = (text, window, cx);
        })
        .on_submit(submit)
        .h(px(height))
        .into_any_element()
    })
    .into_any_element()
}

/// A read-only `.ui-input` box: the value the source renders and cannot edit
/// (a locked official field the service owns, a computed readout).
pub(crate) fn locked_box(
    key: impl Into<SharedString>,
    value: impl Into<SharedString>,
    height: f32,
) -> AnyElement {
    input_field(ElementId::Name(key.into()), value, false)
        .h(px(height))
        .opacity(0.55)
        .cursor_not_allowed()
        .into_any_element()
}

//! `.ui-input` with real text editing.
//!
//! Two constructors:
//!
//! - [`text_field`] for free text (printer name, IP, notes, search).
//! - [`number_field`] / [`number_field_with`] for the numeric fields the panels
//!   actually use: locale formatting through [`gpui_locale`], parse-on-commit,
//!   crash-free rejection of non-numbers and optional clamping.
//!
//! ## Why a custom element
//!
//! A `div` cannot read the [`TextInput`] entity without an `App`, and this
//! module's callers build widgets without threading one in. A custom element
//! gets `App` in prepaint/paint, so it can read the focus handle, register the
//! focus node and install the platform input handler in one place.
//!
//! ## Look
//!
//! `src/app/globals.css` `.ui-input` (lines 320–329 and 342–346):
//! `border: 1px solid var(--border-subtle)`, `background: var(--surface-0)`,
//! `color: var(--text-strong)`, `border-radius: var(--radius-sm)`,
//! `padding: 6px 10px`, `font-size: var(--font-size-sm)`, `line-height: 1.3`,
//! and `border-color: var(--accent)` on focus. The placeholder uses
//! `text-muted` (`--text-muted`), matching the `text-text-muted` utilities the
//! source inputs use for hints. `.ui-input` is unlayered in `globals.css`, so
//! these values win over utility classes on an `<input>`; the port keeps them.
//!
//! ## GPUI input plumbing
//!
//! Typed characters, IME composition and platform paste arrive through
//! [`gpui::Window::handle_input`] with GPUI's own
//! `ElementInputHandler<TextInput>`, because [`TextInput`] implements
//! [`gpui::EntityInputHandler`]. Editing keys (backspace/delete, arrows,
//! Home/End, shift-selection, select-all, cut/copy/paste, word deletion) are
//! key-down listeners registered on this element in paint; they do not need
//! application-level action bindings, so a panel can drop the field in without
//! touching the keymap.
//!
//! Enter is deliberately left to the element: with an `on_submit` closure it is
//! consumed and calls back, without one it bubbles (so a dialog's default
//! action still sees it).

use std::rc::Rc;

use gpui::{
    App, Bounds, ContentMask, CursorStyle, DispatchPhase, Element, ElementId, ElementInputHandler,
    Entity, GlobalElementId, Hitbox, HitboxBehavior, IntoElement, KeyDownEvent, LayoutId,
    MouseButton, MouseDownEvent, Pixels, SharedString, Style, StyleRefinement, Styled, TextAlign,
    TextRun, UnderlineStyle, Window, fill, point, prelude::*, px, relative, rgb, rgba, size,
};
use gpui_locale::{Locale, current_catalog, fmt};

use crate::text_input::TextInput;
use gpui_core::theme::*;

/// `.ui-input` padding: `6px 10px` from `globals.css`.
const PAD_X: f32 = 10.0;
const PAD_Y: f32 = 6.0;
/// `.ui-input` border is 1px and the quad is drawn border-box.
const BORDER: f32 = 1.0;
/// The line box: `font-size: 13px; line-height: 1.3`.
const LINE_HEIGHT: f32 = FONT_SM * 1.3;
/// Selection fill over the surface: the accent at 28% alpha, so the text stays
/// readable on `--surface-0` (the source relies on the platform's selection
/// colour; there is no token for it).
const SELECTION: u32 = (ACCENT << 8) | 0x48;

type ChangeCallback = Rc<dyn Fn(&str, &mut Window, &mut App)>;
type SubmitCallback = Rc<dyn Fn(&str, &mut Window, &mut App)>;
type FrameCallback = Rc<dyn Fn(&Entity<TextInput>, Option<bool>, &mut Window, &mut App)>;

// ── Public constructors ─────────────────────────────────────────────────────

/// An editable `.ui-input`.
///
/// `on_change` fires after every edit — typing, IME commit, paste, deletion.
/// Call [`TextField::on_submit`] to also consume Enter.
///
/// Panels normally pass `cx.listener(..)`-shaped closures:
///
/// ```ignore
/// text_field(&self.name, "Printer name", cx.listener(
///     |this, value: &str, _window, cx| {
///         this.draft.name = value.to_owned();
///         cx.notify();
///     },
/// ))
/// ```
pub fn text_field(
    input: &Entity<TextInput>,
    placeholder: impl Into<SharedString>,
    on_change: impl Fn(&str, &mut Window, &mut App) + 'static,
) -> TextField {
    TextField {
        input: input.clone(),
        placeholder: placeholder.into(),
        on_change: Rc::new(on_change),
        on_submit: None,
        style: StyleRefinement::default(),
    }
}

/// A numeric `.ui-input` in the style of `src/components/ui/NumberInput.tsx`.
///
/// The callback receives the parsed value only when the text parses; anything
/// else restores the displayed value on blur. `unit` is
/// `Some((suffix, scale))`: the field shows `value * scale` followed by
/// `suffix` (a stored 0.05 mm shown as `50 µm` is `Some(("µm", 1000.0))`), and
/// parses back by dividing. The callback always reports the stored unit.
///
/// Use [`NumberField::min`]/[`NumberField::max`] (stored units) and
/// [`NumberField::precision`] to bound and round the value. Rounding and
/// decimal-separator handling go through [`gpui_locale`], never `format!`.
///
/// This plain form takes `Fn(f32)`, so the closure cannot touch view state;
/// panels should normally use [`number_field_with`] with `cx.listener(..)`.
pub fn number_field(
    input: &Entity<TextInput>,
    value: f32,
    unit: Option<(&str, f32)>,
    on_change: impl Fn(f32) + 'static,
) -> NumberField {
    number_field_with(input, value, unit, move |value, _window, _cx| {
        on_change(value)
    })
}

/// [`number_field`] with an `App`-aware callback, for panels.
pub fn number_field_with(
    input: &Entity<TextInput>,
    value: f32,
    unit: Option<(&str, f32)>,
    on_change: impl Fn(f32, &mut Window, &mut App) + 'static,
) -> NumberField {
    let (suffix, scale) = match unit {
        Some((suffix, scale)) if scale.is_finite() && scale > 0.0 => {
            (Some(SharedString::from(suffix)), scale)
        }
        Some((suffix, _)) => (Some(SharedString::from(suffix)), 1.0),
        None => (None, 1.0),
    };
    NumberField {
        input: input.clone(),
        value,
        suffix,
        scale,
        precision: 2,
        min: None,
        max: None,
        on_change: Rc::new(on_change),
        style: StyleRefinement::default(),
    }
}

// ── Formatting / parsing helpers ────────────────────────────────────────────

/// `value` as an editable field shows it: exactly `precision` fraction digits
/// in the active locale, without thousands grouping.
///
/// Formatting goes through [`gpui_locale::fmt::fixed`]; the grouping separators
/// `fixed` adds are stripped because a field should not show `1,234.50` and
/// [`gpui_locale::fmt::parse`] cannot read the group separator back.
pub fn format_number(locale: &Locale, value: f32, precision: usize) -> SharedString {
    let formatted = fmt::fixed(locale, value as f64, precision);
    let group = locale.group_separator();
    if group.is_empty() || group == locale.decimal_separator() {
        formatted
    } else {
        formatted.to_string().replace(group, "").into()
    }
}

/// Rounds `value` to `precision` fraction digits with the locale's
/// round-half-away rule (the same path as displaying then re-reading it).
pub fn round_number(locale: &Locale, value: f32, precision: usize) -> f32 {
    if !value.is_finite() {
        return value;
    }
    fmt::parse(locale, &format_number(locale, value, precision)) as f32
}

/// Reads the number a field holds on commit.
///
/// `text` is parsed with the locale's decimal separator (`.` also works, as
/// [`gpui_locale::fmt::parse`] documents). Empty text or non-numbers restore
/// `previous`; the result is rounded to `precision` and clamped into
/// `min..=max`.
pub fn parse_number(
    locale: &Locale,
    text: &str,
    previous: f32,
    min: Option<f32>,
    max: Option<f32>,
    precision: usize,
) -> f32 {
    let parsed = fmt::parse(locale, text) as f32;
    if !parsed.is_finite() {
        return previous;
    }

    let lower = min.unwrap_or(f32::NEG_INFINITY);
    let upper = max.unwrap_or(f32::INFINITY);
    if lower > upper {
        return previous;
    }

    let rounded = round_number(locale, parsed, precision);
    if !rounded.is_finite() {
        return previous;
    }
    rounded.clamp(lower, upper)
}

/// The active UI locale, as set by `gpui_locale::set_locale` for this thread.
fn active_locale() -> Locale {
    current_catalog().locale().clone()
}

// ── text_field ──────────────────────────────────────────────────────────────

/// The element [`text_field`] returns; add [`Self::on_submit`] for Enter.
pub struct TextField {
    input: Entity<TextInput>,
    placeholder: SharedString,
    on_change: ChangeCallback,
    on_submit: Option<SubmitCallback>,
    style: StyleRefinement,
}

impl TextField {
    /// Called with the field's text when Enter is pressed. Enter is consumed
    /// only when this is set.
    pub fn on_submit(mut self, submit: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_submit = Some(Rc::new(submit));
        self
    }
}

impl Styled for TextField {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl IntoElement for TextField {
    type Element = FieldElement;

    fn into_element(self) -> Self::Element {
        FieldElement {
            spec: FieldSpec {
                input: self.input,
                placeholder: self.placeholder,
                suffix: None,
                install_change: Some(self.on_change),
                on_submit: self.on_submit,
                on_frame: None,
            },
            style: self.style,
        }
    }
}

// ── number_field ────────────────────────────────────────────────────────────

/// The element [`number_field`] returns; bound and round the value with the
/// builders.
pub struct NumberField {
    input: Entity<TextInput>,
    value: f32,
    suffix: Option<SharedString>,
    scale: f32,
    precision: usize,
    min: Option<f32>,
    max: Option<f32>,
    on_change: Rc<dyn Fn(f32, &mut Window, &mut App)>,
    style: StyleRefinement,
}

impl NumberField {
    /// Smallest stored value the field accepts.
    pub fn min(mut self, min: f32) -> Self {
        self.min = Some(min);
        self
    }

    /// Largest stored value the field accepts.
    pub fn max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }

    /// Fraction digits shown and rounds to; two by default, matching
    /// `NumberInput`'s `fmt.field(value, 2)`.
    pub fn precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }
}

impl Styled for NumberField {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl IntoElement for NumberField {
    type Element = FieldElement;

    fn into_element(self) -> Self::Element {
        let locale = active_locale();
        let input = self.input.clone();
        let value = self.value;
        let scale = self.scale;
        let precision = self.precision;
        let min = self.min;
        let max = self.max;
        let on_change = self.on_change.clone();

        // Commit: parse the displayed text, restore the previous value when it
        // is not a number, clamp, round, show the canonical form and report the
        // stored-unit value only when it changed.
        let commit: Rc<dyn Fn(&mut Window, &mut App)> = {
            let locale = locale.clone();
            let input = input.clone();
            Rc::new(move |window: &mut Window, cx: &mut App| {
                let text = input.read(cx).value().to_owned();
                let previous_display = value * scale;
                let displayed = parse_number(
                    &locale,
                    &text,
                    previous_display,
                    min.map(|min| min * scale),
                    max.map(|max| max * scale),
                    precision,
                );
                let next = displayed / scale;
                let canonical = format_number(&locale, displayed, precision);
                input.update(cx, |field, cx| {
                    field.set_display_value(canonical.to_string());
                    cx.notify();
                });
                if next != value {
                    on_change(next, window, cx);
                }
            })
        };

        // Every prepaint: a focus loss commits (restoring a rejected edit), and
        // while unfocused the field mirrors the value the caller passed, which
        // is how an external reset or stepper reaches the text.
        let on_frame: FrameCallback = {
            let locale = locale.clone();
            let commit = commit.clone();
            let input = input.clone();
            Rc::new(move |entity, transition, window, cx| {
                if transition == Some(false) {
                    commit(window, cx);
                } else if !entity.read(cx).is_focused(window) {
                    let display = format_number(&locale, value * scale, precision);
                    input.update(cx, |field, _| field.set_display_value(display.to_string()));
                }
            })
        };

        let on_submit: SubmitCallback = {
            let commit = commit.clone();
            Rc::new(move |_value: &str, window: &mut Window, cx: &mut App| commit(window, cx))
        };

        FieldElement {
            spec: FieldSpec {
                input: self.input,
                placeholder: SharedString::default(),
                suffix: self.suffix,
                install_change: None,
                on_submit: Some(on_submit),
                on_frame: Some(on_frame),
            },
            style: self.style,
        }
    }
}

// ── the element ─────────────────────────────────────────────────────────────

#[derive(Clone)]
struct FieldSpec {
    input: Entity<TextInput>,
    placeholder: SharedString,
    suffix: Option<SharedString>,
    /// Installed on the entity every prepaint so platform-typed characters
    /// reach the caller's change callback.
    install_change: Option<ChangeCallback>,
    on_submit: Option<SubmitCallback>,
    on_frame: Option<FrameCallback>,
}

pub struct FieldPrepaint {
    line: gpui::ShapedLine,
    suffix: Option<gpui::ShapedLine>,
    text_bounds: Bounds<Pixels>,
    hitbox: Hitbox,
}

/// The `.ui-input` box, its text, selection, caret and input plumbing.
pub struct FieldElement {
    spec: FieldSpec,
    style: StyleRefinement,
}

impl Element for FieldElement {
    type RequestLayoutState = ();
    type PrepaintState = Option<FieldPrepaint>;

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
        let mut style = Style::default();
        // Border-box: text line + vertical padding + 1px borders. Callers can
        // override the width (and height) through the returned type's
        // `Styled` methods; full width matches an `<input>` in a flex column.
        style.size.width = relative(1.0).into();
        style.size.height = px(LINE_HEIGHT + 2.0 * (PAD_Y + BORDER)).into();
        style.refine(&self.style);
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let focus_handle = self.spec.input.read(cx).focus_handle();
        window.set_focus_handle(&focus_handle, cx);
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);

        // A field element owns the frame hooks: the change callback has to be
        // on the entity before the platform can deliver a character, and the
        // number field's blur commit has to observe this frame's focus.
        let install_change = self.spec.install_change.clone();
        self.spec
            .input
            .update(cx, |field, _| field.set_on_change(install_change));
        let transition = self
            .spec
            .input
            .update(cx, |field, _| field.note_focus(&*window));
        if let Some(on_frame) = self.spec.on_frame.clone() {
            on_frame(&self.spec.input, transition, window, cx);
        }

        let style = window.text_style();
        let font = style.font();
        let base = TextRun {
            len: 0,
            font,
            color: rgb(TEXT_STRONG).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let (display, text_color, marked) = {
            let field = self.spec.input.read(cx);
            if field.value().is_empty() {
                (self.spec.placeholder.clone(), rgb(TEXT_MUTED), None)
            } else {
                (
                    SharedString::from(field.value().to_owned()),
                    rgb(TEXT_STRONG),
                    field.model().marked_range(),
                )
            }
        };

        let runs = match marked {
            Some(range) if !range.is_empty() && range.end <= display.len() => vec![
                TextRun {
                    len: range.start,
                    color: text_color.into(),
                    ..base.clone()
                },
                TextRun {
                    len: range.end - range.start,
                    color: text_color.into(),
                    underline: Some(UnderlineStyle {
                        color: Some(text_color.into()),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..base.clone()
                },
                TextRun {
                    len: display.len() - range.end,
                    color: text_color.into(),
                    ..base.clone()
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect(),
            _ => vec![TextRun {
                len: display.len(),
                color: text_color.into(),
                ..base
            }],
        };

        let line = window
            .text_system()
            .shape_line(display, px(FONT_SM), &runs, None);

        let suffix = self.spec.suffix.as_ref().map(|suffix| {
            let run = TextRun {
                len: suffix.len(),
                font: style.font(),
                color: rgb(TEXT_MUTED).into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            window
                .text_system()
                .shape_line(suffix.clone(), px(FONT_XS), &[run], None)
        });
        let suffix_width = suffix
            .as_ref()
            .map(|line| line.width() + px(4.0))
            .unwrap_or(px(0.0));

        let text_width = bounds.size.width - px(2.0 * (PAD_X + BORDER)) - suffix_width;
        let text_bounds = Bounds::new(
            point(
                bounds.left() + px(PAD_X + BORDER),
                bounds.top() + px(PAD_Y + BORDER),
            ),
            size(
                px(text_width.0.max(0.0)),
                bounds.size.height - px(2.0 * (PAD_Y + BORDER)),
            ),
        );

        Some(FieldPrepaint {
            line,
            suffix,
            text_bounds,
            hitbox,
        })
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(FieldPrepaint {
            line,
            suffix,
            text_bounds,
            hitbox,
        }) = prepaint.take()
        else {
            return;
        };

        let focus_handle = self.spec.input.read(cx).focus_handle();
        let focused = focus_handle.is_focused(window);
        let (selection, caret) = {
            let field = self.spec.input.read(cx);
            (field.model().selected_range(), field.model().caret_offset())
        };

        // Frame: `.ui-input` border/bg/radius, accent at focus.
        window.paint_quad(
            fill(bounds, rgb(SURFACE_0))
                .corner_radii(px(RADIUS_SM))
                .border_widths(px(BORDER))
                .border_color(rgb(if focused { ACCENT } else { BORDER_SUBTLE })),
        );

        // Typed characters, IME and platform paste reach the entity here.
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.spec.input.clone()),
            cx,
        );

        if focused && !selection.is_empty() {
            window.paint_quad(fill(
                Bounds::from_corners(
                    point(
                        text_bounds.left() + line.x_for_index(selection.start),
                        text_bounds.top(),
                    ),
                    point(
                        text_bounds.left() + line.x_for_index(selection.end),
                        text_bounds.bottom(),
                    ),
                ),
                rgba(SELECTION),
            ));
        }

        // The text is clipped to the padded content box, like `overflow: hidden`
        // on the source `<input>`.
        window.with_content_mask(
            Some(ContentMask {
                bounds: text_bounds,
            }),
            |window| {
                let _ = line.paint(
                    text_bounds.origin,
                    text_bounds.size.height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            },
        );

        if let Some(suffix) = suffix {
            let _ = suffix.paint(
                point(text_bounds.right() + px(4.0), text_bounds.top()),
                text_bounds.size.height,
                TextAlign::Left,
                None,
                window,
                cx,
            );
        }

        if focused && selection.is_empty() {
            // The caret is a block: a 1px column the full line height.
            window.paint_quad(fill(
                Bounds::new(
                    point(
                        text_bounds.left() + line.x_for_index(caret),
                        text_bounds.top(),
                    ),
                    size(px(1.0), text_bounds.size.height),
                ),
                rgb(TEXT_STRONG),
            ));
        }

        if hitbox.is_hovered(window) {
            window.set_cursor_style(CursorStyle::IBeam, &hitbox);
        }

        // Click focuses and places the caret where the pointer landed.
        {
            let input = self.spec.input.clone();
            let focus_handle = focus_handle.clone();
            let line = line.clone();
            let hitbox = hitbox.clone();
            let text_bounds = text_bounds;
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble
                    || event.button != MouseButton::Left
                    || !hitbox.is_hovered(window)
                    || window.default_prevented()
                {
                    return;
                }
                let empty = input.read(cx).value().is_empty();
                let index = if empty {
                    0
                } else {
                    line.closest_index_for_x(event.position.x - text_bounds.left())
                };
                input.update(cx, |field, cx| field.set_caret(index, cx));
                window.focus(&focus_handle, cx);
                window.prevent_default();
            });
        }

        // Editing keys; no application key bindings are needed for these.
        {
            let input = self.spec.input.clone();
            let on_submit = self.spec.on_submit.clone();
            window.on_key_event(move |event: &KeyDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble {
                    return;
                }
                if event.keystroke.key.eq_ignore_ascii_case("enter") {
                    if let Some(on_submit) = on_submit.clone() {
                        let value = input.read(cx).value().to_owned();
                        on_submit(&value, window, cx);
                        cx.stop_propagation();
                    }
                    return;
                }
                input.update(cx, |field, cx| {
                    field.handle_key(event, window, cx);
                });
            });
        }

        // Geometry for IME candidate windows and point-to-index queries.
        self.spec
            .input
            .update(cx, |field, _| field.set_last_line(line, bounds));
    }
}

impl IntoElement for FieldElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn german() -> Locale {
        Locale::parse("de")
    }

    #[test]
    fn format_uses_locale_separator_and_fixed_digits() {
        assert_eq!(
            format_number(&Locale::english(), 1.5, 2).to_string(),
            "1.50"
        );
        assert_eq!(format_number(&german(), 1.5, 2).to_string(), "1,50");
    }

    #[test]
    fn format_strips_thousands_grouping() {
        // `fixed` groups 1234.5 as `1,234.50`; an editable field must not.
        assert_eq!(
            format_number(&Locale::english(), 1234.5, 2).to_string(),
            "1234.50"
        );
        assert_eq!(format_number(&german(), 1234.5, 2).to_string(), "1234,50");
    }

    #[test]
    fn round_is_half_away_from_zero_at_the_locale() {
        assert_eq!(round_number(&Locale::english(), 2.345, 2), 2.35);
        assert_eq!(round_number(&german(), -2.345, 2), -2.35);
        assert_eq!(round_number(&Locale::english(), 1.0, 2), 1.0);
    }

    #[test]
    fn parse_reads_the_locale_separator() {
        assert_eq!(parse_number(&german(), "12,345", 0.0, None, None, 2), 12.35);
        assert_eq!(
            parse_number(&Locale::english(), "12.345", 0.0, None, None, 2),
            12.35
        );
        // A dot is accepted alongside the locale separator.
        assert_eq!(parse_number(&german(), "1.25", 0.0, None, None, 2), 1.25);
    }

    #[test]
    fn parse_rejects_non_numbers_with_the_previous_value() {
        assert_eq!(
            parse_number(&Locale::english(), "abc", 4.0, None, None, 2),
            4.0
        );
        assert_eq!(
            parse_number(&Locale::english(), "", 4.0, None, None, 2),
            4.0
        );
        assert_eq!(
            parse_number(&Locale::english(), "-", 4.0, None, None, 2),
            4.0
        );
        assert_eq!(
            parse_number(&Locale::english(), "1,5", 4.0, None, None, 2),
            4.0
        );
    }

    #[test]
    fn parse_clamps_into_the_bounds() {
        assert_eq!(
            parse_number(&Locale::english(), "20", 0.0, Some(0.0), Some(10.0), 2),
            10.0
        );
        assert_eq!(
            parse_number(&Locale::english(), "-5", 0.0, Some(0.0), None, 2),
            0.0
        );
        // A bound range that is inverted is treated as a rejected edit rather
        // than panicking in `clamp`.
        assert_eq!(
            parse_number(&Locale::english(), "5", 3.0, Some(10.0), Some(0.0), 2),
            3.0
        );
    }

    #[test]
    fn parse_rounds_before_clamping() {
        assert_eq!(parse_number(&german(), "2,349", 0.0, None, None, 2), 2.35);
        // 0.125 is exact in both f32 and f64, so the half-away rule is
        // unambiguous here.
        assert_eq!(
            parse_number(&Locale::english(), "0.125", 0.0, None, None, 2),
            0.13
        );
    }
}

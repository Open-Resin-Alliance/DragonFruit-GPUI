//! The editing state behind [`widgets::text_field`](crate::widgets::text_field).
//!
//! GPUI delivers typed characters through the platform's text-input path:
//! [`Window::handle_input`] installs an [`EntityInputHandler`] for the focused
//! element and the platform then calls `replace_text_in_range` for every
//! character, every IME commit and every platform-initiated paste. That is the
//! path this entity implements; the drawing lives in
//! [`widgets::text_field`](crate::widgets::text_field), because it needs the
//! element bounds and a shaped line.
//!
//! The editing rules themselves are in [`TextModel`], a plain value with no
//! GPUI types, so insertion, selection, caret movement, word deletion and the
//! UTF-16 conversions GPUI's IME protocol speaks can be tested without a
//! window. [`TextInput`] is that model plus the focus handle, the last shaped
//! line (for IME candidate placement and mouse hit-testing) and the change
//! hook.
//!
//! What the entity does **not** do is decide when it is shown; a
//! `text_field(..)` element installs the change hook during prepaint and asks
//! [`TextInput::note_focus`] about focus transitions, so blur commits and
//! submit handling stay in the view.
//!
//! Conventions taken from the source `Input`/`NumberInput` components
//! (`src/components/atoms/Input.tsx`, `src/components/ui/NumberInput.tsx`):
//! single-line text only, newlines pasted into the field become spaces, and
//! the caret sits at the end when a value is set programmatically.

use std::ops::Range;
use std::rc::Rc;

use gpui::{
    App, Bounds, ClipboardItem, Context, EntityInputHandler, FocusHandle, KeyDownEvent, Pixels,
    Point, ShapedLine, TextInputAction, TextInputConfiguration, UTF16Selection, Window, px,
    prelude::*,
};

/// A change hook installed by a field element on every prepaint.
///
/// The closure owns the value string, so it may update panel state through
/// `App` (a `cx.listener(..)` composes directly).
pub type FieldChangeCallback = Rc<dyn Fn(&str, &mut Window, &mut App)>;

/// The editing model: text, selection and IME composition range in byte
/// offsets, with no GPUI dependency.
///
/// Offsets are UTF-8 byte indices and always lie on a character boundary.
/// The selection is stored as a half-open range plus a direction, matching
/// the ordering GPUI's [`UTF16Selection`] wants back.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextModel {
    content: String,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
}

impl TextModel {
    /// An empty model.
    pub fn new() -> Self {
        Self::default()
    }

    /// A model for `content`, with the caret after the last character — what a
    /// freshly populated field shows.
    pub fn with_content(content: impl Into<String>) -> Self {
        let content = content.into();
        let caret = content.len();
        Self {
            content,
            selected_range: caret..caret,
            selection_reversed: false,
            marked_range: None,
        }
    }

    /// The field's text.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// The selected range; empty when the caret is collapsed.
    pub fn selected_range(&self) -> Range<usize> {
        self.selected_range.clone()
    }

    /// Whether the selection was made right-to-left, so the caret is at its
    /// start.
    pub fn selection_reversed(&self) -> bool {
        self.selection_reversed
    }

    /// The composing (uncommitted IME) range, if any.
    pub fn marked_range(&self) -> Option<Range<usize>> {
        self.marked_range.clone()
    }

    /// The caret: the moving end of the selection.
    pub fn caret_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    /// Replaces the text and puts the caret at the end. Programmatic writes do
    /// not fire change hooks; the caller owns the model.
    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
        let caret = self.content.len();
        self.selected_range = caret..caret;
        self.selection_reversed = false;
        self.marked_range = None;
    }

    /// Collapses the selection to `offset`.
    pub fn move_to(&mut self, offset: usize) {
        let offset = self.clamp_offset(offset);
        self.selected_range = offset..offset;
        self.selection_reversed = false;
    }

    /// Extends the selection to `offset`, keeping the anchor where it was.
    pub fn select_to(&mut self, offset: usize) {
        let offset = self.clamp_offset(offset);
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
    }

    /// Selects everything.
    pub fn select_all(&mut self) {
        self.selected_range = 0..self.content.len();
        self.selection_reversed = false;
    }

    /// Sets the selection directly, as IME composition does.
    pub fn set_selection(&mut self, range: Range<usize>) {
        let start = self.clamp_offset(range.start);
        let end = self.clamp_offset(range.end).max(start);
        self.selected_range = start..end;
        self.selection_reversed = false;
    }

    /// The character before `offset`.
    pub fn previous_boundary(&self, offset: usize) -> usize {
        let offset = self.clamp_offset(offset);
        self.content
            .char_indices()
            .rev()
            .find_map(|(index, _)| (index < offset).then_some(index))
            .unwrap_or(0)
    }

    /// The character after `offset`.
    pub fn next_boundary(&self, offset: usize) -> usize {
        let offset = self.clamp_offset(offset);
        self.content
            .char_indices()
            .find_map(|(index, character)| {
                (index > offset).then_some(index + character.len_utf8())
            })
            .unwrap_or(self.content.len())
    }

    /// The start of the word before `offset`: trailing whitespace is skipped,
    /// then a run of non-whitespace. Punctuation counts as part of the word, as
    /// a single-line field has no lexical syntax to respect.
    pub fn previous_word_boundary(&self, offset: usize) -> usize {
        let mut index = self.clamp_offset(offset);
        while index > 0 {
            let previous = self.previous_boundary(index);
            let character = self.content[previous..index]
                .chars()
                .next()
                .unwrap_or(' ');
            if character.is_whitespace() {
                index = previous;
            } else {
                break;
            }
        }
        while index > 0 {
            let previous = self.previous_boundary(index);
            let character = self.content[previous..index]
                .chars()
                .next()
                .unwrap_or(' ');
            if !character.is_whitespace() {
                index = previous;
            } else {
                break;
            }
        }
        index
    }

    /// The end of the word after `offset`; the mirror of
    /// [`Self::previous_word_boundary`].
    pub fn next_word_boundary(&self, offset: usize) -> usize {
        let mut index = self.clamp_offset(offset);
        while index < self.content.len() {
            let next = self.next_boundary(index);
            let character = self.content[index..next].chars().next().unwrap_or(' ');
            if character.is_whitespace() {
                index = next;
            } else {
                break;
            }
        }
        while index < self.content.len() {
            let next = self.next_boundary(index);
            let character = self.content[index..next].chars().next().unwrap_or(' ');
            if !character.is_whitespace() {
                index = next;
            } else {
                break;
            }
        }
        index
    }

    /// Replaces `range` with `text` and leaves the caret after it.
    ///
    /// Returns the byte offset where the inserted text starts.
    pub fn replace_range(&mut self, range: Range<usize>, text: &str) -> usize {
        let start = self.clamp_offset(range.start);
        let end = self.clamp_offset(range.end).max(start);
        let mut content = String::with_capacity(self.content.len() + text.len());
        content.push_str(&self.content[..start]);
        content.push_str(text);
        content.push_str(&self.content[end..]);
        self.content = content;
        let caret = start + text.len();
        self.selected_range = caret..caret;
        self.selection_reversed = false;
        self.marked_range = None;
        caret
    }

    /// Inserts at the selection (or at the marked range while composing).
    pub fn insert(&mut self, text: &str) -> usize {
        let range = self
            .marked_range
            .clone()
            .unwrap_or_else(|| self.selected_range.clone());
        self.replace_range(range, text)
    }

    /// Inserts in an explicit range while recording it as the IME composing
    /// range. Returns where the text starts.
    pub fn replace_and_mark(&mut self, range: Range<usize>, text: &str) -> usize {
        let start = self.replace_range(range, text);
        if !text.is_empty() {
            self.marked_range = Some(start..start + text.len());
        }
        start
    }

    /// Clears the composing range.
    pub fn clear_marked(&mut self) {
        self.marked_range = None;
    }

    /// Backspace: deletes the selection, or the character before the caret.
    pub fn delete_backward(&mut self) -> bool {
        if !self.selected_range.is_empty() {
            self.replace_range(self.selected_range.clone(), "");
            return true;
        }
        let caret = self.caret_offset();
        let previous = self.previous_boundary(caret);
        if previous == caret {
            return false;
        }
        self.replace_range(previous..caret, "");
        true
    }

    /// Delete: removes the selection, or the character after the caret.
    pub fn delete_forward(&mut self) -> bool {
        if !self.selected_range.is_empty() {
            self.replace_range(self.selected_range.clone(), "");
            return true;
        }
        let caret = self.caret_offset();
        let next = self.next_boundary(caret);
        if next == caret {
            return false;
        }
        self.replace_range(caret..next, "");
        true
    }

    /// Ctrl/Alt+Backspace.
    pub fn delete_word_backward(&mut self) -> bool {
        if !self.selected_range.is_empty() {
            self.replace_range(self.selected_range.clone(), "");
            return true;
        }
        let caret = self.caret_offset();
        let previous = self.previous_word_boundary(caret);
        if previous == caret {
            return false;
        }
        self.replace_range(previous..caret, "");
        true
    }

    /// Ctrl/Alt+Delete.
    pub fn delete_word_forward(&mut self) -> bool {
        if !self.selected_range.is_empty() {
            self.replace_range(self.selected_range.clone(), "");
            return true;
        }
        let caret = self.caret_offset();
        let next = self.next_word_boundary(caret);
        if next == caret {
            return false;
        }
        self.replace_range(caret..next, "");
        true
    }

    /// UTF-8 byte offset to the equivalent UTF-16 index.
    pub fn offset_to_utf16(&self, offset: usize) -> usize {
        offset_to_utf16(&self.content, offset)
    }

    /// UTF-16 index to a UTF-8 byte offset, snapped to a character boundary.
    pub fn offset_from_utf16(&self, offset: usize) -> usize {
        offset_from_utf16(&self.content, offset)
    }

    /// A UTF-8 range as UTF-16 indices.
    pub fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    /// A UTF-16 range as UTF-8 offsets.
    pub fn range_from_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range.start)..self.offset_from_utf16(range.end)
    }

    fn clamp_offset(&self, offset: usize) -> usize {
        let mut offset = offset.min(self.content.len());
        while offset > 0 && !self.content.is_char_boundary(offset) {
            offset -= 1;
        }
        offset
    }
}

/// UTF-8 byte offset to UTF-16 index.
pub fn offset_to_utf16(text: &str, offset: usize) -> usize {
    let mut utf16_offset = 0;
    let mut utf8_count = 0;
    for character in text.chars() {
        if utf8_count >= offset {
            break;
        }
        utf8_count += character.len_utf8();
        utf16_offset += character.len_utf16();
    }
    utf16_offset
}

/// UTF-16 index to a UTF-8 byte offset, never splitting a character.
pub fn offset_from_utf16(text: &str, offset: usize) -> usize {
    let mut utf8_offset = 0;
    let mut utf16_count = 0;
    for character in text.chars() {
        if utf16_count >= offset {
            break;
        }
        utf16_count += character.len_utf16();
        utf8_offset += character.len_utf8();
    }
    utf8_offset
}

/// The entity a `text_field(..)` element draws and drives.
///
/// The entity implements [`EntityInputHandler`], so GPUI's standard
/// `ElementInputHandler` can forward the platform's text input to it through
/// [`Window::handle_input`]. Editing keys (backspace, arrows, Home/End,
/// select-all, cut/copy/paste, word deletion) arrive as key-down events and are
/// handled by [`Self::handle_key`]; typed characters and IME commits arrive
/// through the input-handler trait, not through key events.
pub struct TextInput {
    model: TextModel,
    focus_handle: FocusHandle,
    last_focused: bool,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    on_change: Option<FieldChangeCallback>,
}

impl TextInput {
    /// A field showing `value` with the caret after it.
    pub fn new(cx: &mut Context<Self>, value: impl Into<String>) -> Self {
        Self {
            model: TextModel::with_content(value),
            focus_handle: cx.focus_handle(),
            last_focused: false,
            last_layout: None,
            last_bounds: None,
            on_change: None,
        }
    }

    /// The editing model, for drawing and for tests.
    pub fn model(&self) -> &TextModel {
        &self.model
    }

    /// The current text.
    pub fn value(&self) -> &str {
        self.model.content()
    }

    /// The focus handle the field element tracks.
    pub fn focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    /// Replaces the text from code (a draft being reset, a stepper writing a
    /// value). Does not fire the change hook.
    pub fn set_value(&mut self, value: impl Into<String>, cx: &mut Context<Self>) {
        self.model.set_content(value);
        cx.notify();
    }

    /// [`Self::set_value`] without a repaint request, for a view that is
    /// already painting this field (the number field's format sync).
    pub fn set_display_value(&mut self, value: impl Into<String>) {
        let value = value.into();
        if self.model.content() != value {
            self.model.set_content(value);
        }
    }

    /// Installs the hook a field element wants called after every edit.
    ///
    /// The element does this on prepaint, so the hook is in place before the
    /// platform can deliver a character to the focused input handler.
    pub fn set_on_change(&mut self, callback: Option<FieldChangeCallback>) {
        self.on_change = callback;
    }

    /// Compares the field's focus with the previous prepaint and reports a
    /// true focus transition.
    ///
    /// `Some(true)` means the field just gained focus, `Some(false)` that it
    /// just lost it (the number field commits there), `None` that nothing
    /// changed.
    pub fn note_focus(&mut self, window: &Window) -> Option<bool> {
        let focused = self.focus_handle.is_focused(window);
        let changed = (focused != self.last_focused).then_some(focused);
        self.last_focused = focused;
        changed
    }

    /// Whether the field has focus in `window`.
    pub fn is_focused(&self, window: &Window) -> bool {
        self.focus_handle.is_focused(window)
    }

    /// Records the line the element painted, so IME candidate placement and
    /// point-to-index queries have geometry.
    pub fn set_last_line(&mut self, line: ShapedLine, bounds: Bounds<Pixels>) {
        self.last_layout = Some(line);
        self.last_bounds = Some(bounds);
    }

    /// Handles one key-down event. Returns whether the field consumed it.
    ///
    /// Only editing keys are handled here. A printable key's character comes
    /// through [`EntityInputHandler::replace_text_in_range`] separately, so
    /// inserting `keystroke.key_char` here would double-type it.
    pub fn handle_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let keystroke = &event.keystroke;
        let key = keystroke.key.to_ascii_lowercase();
        let primary = keystroke.modifiers.platform || keystroke.modifiers.control;
        let shift = keystroke.modifiers.shift;

        let handled = match key.as_str() {
            "backspace" if primary || keystroke.modifiers.alt => {
                self.model.delete_word_backward();
                self.after_edit(window, cx);
                true
            }
            "delete" if primary || keystroke.modifiers.alt => {
                self.model.delete_word_forward();
                self.after_edit(window, cx);
                true
            }
            "backspace" => {
                if self.model.delete_backward() {
                    self.after_edit(window, cx);
                }
                true
            }
            "delete" => {
                if self.model.delete_forward() {
                    self.after_edit(window, cx);
                }
                true
            }
            "left" => {
                if shift {
                    self.model.select_to(self.model.previous_boundary(self.model.caret_offset()));
                } else if !self.model.selected_range().is_empty() && !shift {
                    self.model.move_to(self.model.selected_range().start);
                } else {
                    self.model
                        .move_to(self.model.previous_boundary(self.model.caret_offset()));
                }
                cx.notify();
                true
            }
            "right" => {
                if shift {
                    self.model.select_to(self.model.next_boundary(self.model.caret_offset()));
                } else if !self.model.selected_range().is_empty() {
                    self.model.move_to(self.model.selected_range().end);
                } else {
                    self.model
                        .move_to(self.model.next_boundary(self.model.caret_offset()));
                }
                cx.notify();
                true
            }
            "home" => {
                if shift {
                    self.model.select_to(0);
                } else {
                    self.model.move_to(0);
                }
                cx.notify();
                true
            }
            "end" => {
                let end = self.model.content().len();
                if shift {
                    self.model.select_to(end);
                } else {
                    self.model.move_to(end);
                }
                cx.notify();
                true
            }
            "a" if primary => {
                self.model.select_all();
                cx.notify();
                true
            }
            "c" if primary => {
                self.copy(cx);
                true
            }
            "x" if primary => {
                self.copy(cx);
                if self.model.delete_backward() {
                    self.after_edit(window, cx);
                }
                true
            }
            "v" if primary => {
                self.paste_from_clipboard(window, cx);
                true
            }
            _ => false,
        };

        // Enter is left to the element: a plain field may let it bubble to the
        // dialog's default action, while a submit-enabled field consumes it.
        if handled {
            cx.stop_propagation();
        }
        handled
    }

    /// Inserts `text` at the selection, replacing it, and fires the change
    /// hook. Newlines become spaces: this is a single-line field.
    pub fn insert_text(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let text = single_line(text);
        self.model.insert(&text);
        self.after_edit(window, cx);
    }

    fn paste_from_clipboard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.insert_text(&text, window, cx);
        }
    }

    fn copy(&self, cx: &mut App) {
        let range = self.model.selected_range();
        if !range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.model.content()[range].to_string(),
            ));
        }
    }

    fn after_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.notify();
        if let Some(callback) = self.on_change.clone() {
            let value = self.model.content().to_owned();
            callback(&value, window, &mut **cx);
        }
    }
}

/// A single-line field cannot hold a line break; the source `Input` is an
/// `<input>`, which does the same sanitisation on paste.
fn single_line(text: &str) -> String {
    text.replace("\r\n", " ").replace(['\r', '\n'], " ")
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.model.range_from_utf16(&range_utf16);
        *adjusted_range = Some(self.model.range_to_utf16(&range));
        Some(self.model.content()[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.model.range_to_utf16(&self.model.selected_range()),
            reversed: self.model.selection_reversed(),
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.model
            .marked_range()
            .as_ref()
            .map(|range| self.model.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.model.clear_marked();
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range| self.model.range_from_utf16(range))
            .or_else(|| self.model.marked_range())
            .unwrap_or_else(|| self.model.selected_range());
        let text = single_line(text);
        self.model.replace_range(range, &text);
        self.after_edit(window, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range| self.model.range_from_utf16(range))
            .or_else(|| self.model.marked_range())
            .unwrap_or_else(|| self.model.selected_range());
        let new_text = single_line(new_text);
        let start = self.model.replace_and_mark(range, &new_text);
        let selection = new_selected_range_utf16
            .map(|range| {
                let inner = offset_from_utf16(&new_text, range.start)
                    ..offset_from_utf16(&new_text, range.end);
                start + inner.start..start + inner.end
            })
            .unwrap_or_else(|| start + new_text.len()..start + new_text.len());
        self.model.set_selection(selection);
        // `set_selection` clears the reversed flag; composing text always moves
        // forward.
        self.after_edit(window, cx);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let line = self.last_layout.as_ref()?;
        let range = self.model.range_from_utf16(&range_utf16);
        Some(Bounds::from_corners(
            Point::new(
                bounds.left() + line.x_for_index(range.start),
                bounds.top(),
            ),
            Point::new(bounds.left() + line.x_for_index(range.end), bounds.bottom()),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.last_bounds.as_ref()?;
        let line = self.last_layout.as_ref()?;
        let index = line.closest_index_for_x(point.x - bounds.left());
        Some(self.model.offset_to_utf16(index))
    }

    fn accepts_text_input(&self, _window: &mut Window, _cx: &mut Context<Self>) -> bool {
        true
    }

    fn text_input_configuration(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> TextInputConfiguration {
        TextInputConfiguration {
            input_action: TextInputAction::Done,
            ..TextInputConfiguration::default()
        }
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(text: &str) -> TextModel {
        TextModel::with_content(text)
    }

    #[test]
    fn new_model_puts_the_caret_at_the_end() {
        let model = model("abc");
        assert_eq!(model.caret_offset(), 3);
        assert!(model.selected_range().is_empty());
    }

    #[test]
    fn insert_at_caret_lands_before_the_trailing_text() {
        let mut model = model("ac");
        model.move_to(1);
        model.insert("b");
        assert_eq!(model.content(), "abc");
        assert_eq!(model.caret_offset(), 2);
    }

    #[test]
    fn insert_replaces_the_selection() {
        let mut model = model("hello world");
        model.set_selection(0..5);
        model.insert("goodbye");
        assert_eq!(model.content(), "goodbye world");
        assert_eq!(model.caret_offset(), 7);
    }

    #[test]
    fn delete_backward_removes_the_character_before_the_caret() {
        let mut model = model("abc");
        assert!(model.delete_backward());
        assert_eq!(model.content(), "ab");
        assert_eq!(model.caret_offset(), 2);
    }

    #[test]
    fn delete_forward_removes_the_character_after_the_caret() {
        let mut model = model("abc");
        model.move_to(1);
        assert!(model.delete_forward());
        assert_eq!(model.content(), "ac");
    }

    #[test]
    fn delete_selection_replaces_the_whole_selection() {
        let mut model = model("abcdef");
        model.set_selection(2..5);
        assert!(model.delete_backward());
        assert_eq!(model.content(), "abf");
        assert_eq!(model.caret_offset(), 2);
    }

    #[test]
    fn arrows_walk_character_boundaries() {
        let mut model = model("héllo");
        // h=0, é=1..3, l=3, l=4, o=5
        model.move_to(3);
        assert_eq!(model.previous_boundary(model.caret_offset()), 1);
        assert_eq!(model.next_boundary(model.caret_offset()), 4);

        model.move_to(0);
        model.select_to(3);
        assert_eq!(model.selected_range(), 0..3);
        assert!(!model.selection_reversed());
    }

    #[test]
    fn select_left_moves_the_anchor_and_reverses_past_it() {
        let mut model = model("abcd");
        model.move_to(2);
        model.select_to(1);
        assert_eq!(model.selected_range(), 1..2);
        assert!(model.selection_reversed());
        assert_eq!(model.caret_offset(), 1);
        model.move_to(4);
        assert!(!model.selection_reversed());
    }

    #[test]
    fn home_and_end_collapse_the_selection() {
        let mut model = model("abc");
        model.move_to(2);
        model.select_to(0);
        model.move_to(0);
        assert_eq!(model.caret_offset(), 0);
        model.move_to(model.content().len());
        assert_eq!(model.caret_offset(), 3);
        assert!(model.selected_range().is_empty());
    }

    #[test]
    fn select_all_then_replace() {
        let mut model = model("old name");
        model.select_all();
        assert_eq!(model.selected_range(), 0..8);
        model.insert("new");
        assert_eq!(model.content(), "new");
        assert_eq!(model.caret_offset(), 3);
    }

    #[test]
    fn paste_at_selection_keeps_the_surroundings() {
        let mut model = model("a[]c");
        model.set_selection(1..3);
        model.insert("b");
        assert_eq!(model.content(), "abc");
        assert_eq!(model.caret_offset(), 2);
    }

    #[test]
    fn word_delete_stops_at_whitespace() {
        let mut model = model("bottle price ");
        assert_eq!(model.previous_word_boundary(13), 7);
        assert!(model.delete_word_backward());
        assert_eq!(model.content(), "bottle ");
        assert_eq!(model.caret_offset(), 7);

        let mut model = model("one two three");
        model.move_to(9);
        assert_eq!(model.previous_word_boundary(9), 4);
        let mut model2 = model.clone();
        assert!(model2.delete_word_backward());
        assert_eq!(model2.content(), "one three");
    }

    #[test]
    fn word_delete_forward_stops_after_the_word() {
        let mut model = model("one two");
        model.move_to(0);
        assert_eq!(model.next_word_boundary(0), 3);
        assert!(model.delete_word_forward());
        assert_eq!(model.content(), " two");
    }

    #[test]
    fn word_boundaries_handle_punctuation_and_edges() {
        let mut model = model("  spaced  ");
        assert_eq!(model.previous_word_boundary(10), 2);
        assert_eq!(model.next_word_boundary(0), 2);
        assert_eq!(model.previous_word_boundary(0), 0);
        assert_eq!(model.next_word_boundary(10), 10);
        model.select_all();
        model.insert("x");
        assert_eq!(model.content(), "x");
    }

    #[test]
    fn utf16_conversions_round_trip_non_ascii() {
        let model = model("aé𝄞");
        assert_eq!(model.offset_to_utf16(0), 0);
        assert_eq!(model.offset_to_utf16(1), 1);
        assert_eq!(model.offset_to_utf16(3), 2);
        assert_eq!(model.offset_to_utf16(7), 4);
        assert_eq!(model.offset_from_utf16(2), 3);
        assert_eq!(model.offset_from_utf16(4), 7);
        // Mid-surrogate indices snap back to a character boundary.
        assert_eq!(model.offset_from_utf16(3), 3);
    }

    #[test]
    fn marking_tracks_the_composition_range() {
        let mut model = model("ab");
        model.set_selection(2..2);
        let start = model.replace_and_mark(2..2, "に");
        assert_eq!(start, 2);
        assert_eq!(model.marked_range(), Some(2..5));
        let start = model.replace_and_mark(2..5, "日本");
        assert_eq!(start, 2);
        assert_eq!(model.content(), "ab日本");
        assert_eq!(model.marked_range(), Some(2..8));
        model.clear_marked();
        assert_eq!(model.marked_range(), None);
    }

    #[test]
    fn single_line_flattens_pasted_breaks() {
        assert_eq!(single_line("a\r\nb\nc\rd"), "a b c d");
    }
}

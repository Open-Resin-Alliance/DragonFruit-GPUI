//! The catalogue the current thread translates through, and what `t!` reads.
//!
//! Lingui's `useLingui()` hands every component a `_` bound to the active
//! locale. GPUI has no provider tree to thread that through, and almost every
//! call site is a `t!("Some label")` that cannot spell the locale, so the
//! active catalogue lives in a thread-local: the UI thread sets it once when a
//! locale is chosen (`set_locale`) and formatting paths that already hold a
//! [`Catalog`] use [`Catalog::text`] directly instead.
//!
//! Keep the scope honest: background fibres are their own threads, so a `t!`
//! there sees the English default unless that thread sets a locale too.

use std::cell::RefCell;

use crate::catalog::Catalog;
use crate::locale::Locale;
use crate::SharedString;

thread_local! {
    static CURRENT: RefCell<Catalog> = RefCell::new(Catalog::english());
}

/// A copy of this thread's catalogue.
pub fn current_catalog() -> Catalog {
    CURRENT.with(|current| current.borrow().clone())
}

/// Replaces this thread's catalogue.
pub fn set_current_catalog(catalog: Catalog) {
    CURRENT.with(|current| *current.borrow_mut() = catalog);
}

/// Replaces this thread's catalogue with `locale`'s.
///
/// Call this when the user switches language; it is the GPUI counterpart of the
/// locale switch that re-renders every `useLingui()` consumer.
pub fn set_locale(locale: &Locale) {
    set_current_catalog(Catalog::for_locale(locale));
}

/// [`Catalog::text`] against this thread's catalogue; the body of [`t!`].
#[doc(hidden)]
pub fn translate(msgid: &'static str) -> SharedString {
    CURRENT.with(|current| current.borrow().get(msgid))
}

//! A BCP-47 language tag, and the number-format facts each shipped tag needs.
//!
//! Ports the locale half of `src/utils/localeNumber.ts` (which reads locale
//! facts out of `Intl.NumberFormat`) and the locale list in
//! `lingui.config.ts` (`en, es, de, fr, sk, cs`, with `en` as the source
//! locale).

use std::fmt;

use crate::SUPPORTED_LANGUAGES;

/// A BCP-47 tag such as `en`, `de-AT` or `zh-Hant-TW`.
///
/// Tags are canonicalised on construction — language lowercase, script
/// titlecase, region uppercase, `-` between subtags — so `de_AT` and `de-at`
/// both become `de-AT`, and lookups keyed on the language subtag behave the
/// same for every region. Case is chosen by subtag length, which is right for
/// languages, scripts and regions; extension and private-use subtags are rare
/// here and only get the same length-based treatment.
///
/// Separators are only known for the shipped catalogues; any other language
/// formats with English separators, matching [`Catalog`](crate::Catalog)'s
/// English fallback. A language with no catalogue shows English text, so its
/// numbers are English too.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Locale {
    tag: String,
}

impl Locale {
    /// The source locale, `en`.
    pub fn english() -> Self {
        Self { tag: String::from("en") }
    }

    /// Parses and canonicalises `tag`.
    ///
    /// Empty or malformed input falls back to English; a tag is accepted when
    /// every subtag is ASCII alphanumeric, which covers the private-use and
    /// extended subtags Lingui and settings profiles can hand us.
    pub fn parse(tag: &str) -> Self {
        let mut subtags = tag
            .trim()
            .split(['-', '_'])
            .filter(|subtag| !subtag.is_empty())
            .peekable();

        if subtags.peek().is_none() {
            return Self::english();
        }

        let mut canonical = String::with_capacity(tag.len());
        for (index, subtag) in subtags.enumerate() {
            if !subtag.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
                // The tag can be a user-editable string in a profile; a bogus
                // one should render English text, not panic.
                return Self::english();
            }

            if index > 0 {
                canonical.push('-');
            }

            let canonical_subtag = if index == 0 {
                subtag.to_ascii_lowercase()
            } else if subtag.len() == 4 {
                // Script subtags are titlecase: `Hant`, `Latn`.
                let mut chars = subtag.chars();
                match chars.next() {
                    Some(first) => {
                        first.to_ascii_uppercase().to_string() + &chars.as_str().to_ascii_lowercase()
                    }
                    None => String::new(),
                }
            } else if subtag.len() == 2 || subtag.len() == 3 {
                // Region subtags are uppercase: `AT`, `419`.
                subtag.to_ascii_uppercase()
            } else {
                subtag.to_ascii_lowercase()
            };

            canonical.push_str(&canonical_subtag);
        }

        Self { tag: canonical }
    }

    /// The canonical tag, e.g. `de-AT`.
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// The language subtag, e.g. `de`.
    pub fn language(&self) -> &str {
        self.tag.split('-').next().unwrap_or("en")
    }

    /// The region subtag, if the tag carries one, e.g. `AT`.
    pub fn region(&self) -> Option<&str> {
        self.tag
            .split('-')
            .skip(1)
            .find(|subtag| subtag.len() == 2 || (subtag.len() == 3 && subtag.bytes().all(|b| b.is_ascii_digit())))
    }

    /// Whether a catalogue ships for this language; compare
    /// [`SUPPORTED_LANGUAGES`].
    pub fn is_supported(&self) -> bool {
        SUPPORTED_LANGUAGES.contains(&self.language())
    }

    /// The character between a number's whole and its fraction, e.g. `,` for
    /// German.
    pub fn decimal_separator(&self) -> &'static str {
        self.data().decimal_separator
    }

    /// The character between digit groups, e.g. `.` for German, a narrow
    /// no-break space for French.
    pub fn group_separator(&self) -> &'static str {
        self.data().group_separator
    }

    /// How many extra integer digits disable grouping.
    ///
    /// `Intl.NumberFormat` groups `1000` as `1,000` in English but leaves
    /// Spanish at `1000` until the fifth digit, which is CLDR's
    /// `minimumGroupingDigits = 2` for `es`.
    pub fn minimum_grouping_digits(&self) -> usize {
        self.data().minimum_grouping_digits
    }

    fn data(&self) -> LocaleData {
        data_for(self.language())
    }
}

impl Default for Locale {
    fn default() -> Self {
        Self::english()
    }
}

impl From<&str> for Locale {
    fn from(tag: &str) -> Self {
        Self::parse(tag)
    }
}

impl From<String> for Locale {
    fn from(tag: String) -> Self {
        Self::parse(&tag)
    }
}

impl AsRef<str> for Locale {
    fn as_ref(&self) -> &str {
        self.tag()
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.tag)
    }
}

/// The number-format data of one language, as `Intl.NumberFormat` reports it.
struct LocaleData {
    decimal_separator: &'static str,
    group_separator: &'static str,
    minimum_grouping_digits: usize,
}

/// The English defaults: `Intl.NumberFormat("en").format(1234567.89)` is
/// `1,234,567.89`.
const ENGLISH: LocaleData = LocaleData {
    decimal_separator: ".",
    group_separator: ",",
    minimum_grouping_digits: 1,
};

/// `Intl.NumberFormat(language).format(1234567.89)` for the shipped catalogues.
///
/// Measured in Node's ICU build, which is what `src/utils/localeNumber.ts`
/// formats through:
///
/// - `es`: `1.234.567,89`, and `1000` stays ungrouped while `12345` becomes
///   `12.345`, so `minimumGroupingDigits` is 2.
/// - `de`: `1.234.567,89`.
/// - `fr`: `1 234 567,89` with U+202F (narrow no-break space) between groups.
/// - `sk`, `cs`: `1 234 567,89` with U+00A0 (no-break space) between groups.
///
/// Everything else formats like English.
fn data_for(language: &str) -> LocaleData {
    match language {
        "es" => LocaleData {
            decimal_separator: ",",
            group_separator: ".",
            minimum_grouping_digits: 2,
        },
        "de" => LocaleData {
            decimal_separator: ",",
            group_separator: ".",
            minimum_grouping_digits: 1,
        },
        "fr" => LocaleData {
            decimal_separator: ",",
            group_separator: "\u{202f}",
            minimum_grouping_digits: 1,
        },
        "sk" | "cs" => LocaleData {
            decimal_separator: ",",
            group_separator: "\u{00a0}",
            minimum_grouping_digits: 1,
        },
        _ => ENGLISH,
    }
}

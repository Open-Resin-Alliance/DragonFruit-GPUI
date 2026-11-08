//! Numbers with units for user-visible text: the port of
//! `src/utils/unitFormatting.ts`.
//!
//! Unit symbols are not universal — Cyrillic, Hebrew and Arabic locales write
//! `мм`, `מ״מ`, `مم`, and locales differ on spacing (`28 %` vs `28%`) — so each
//! unit is one message with a `{value}` placeholder, and translators control
//! both the symbol and where the number goes. The number itself is formatted
//! for the locale by [`crate::fmt`] before it is placed.
//!
//! The TS `UNIT_MESSAGES` object is a closed table keyed by unit name; here it
//! is a closed [`Unit`] enum carrying the same msgid and translator comment, so
//! a typo is a compile error and a new unit is one match arm.

use std::fmt;

use crate::catalog::Catalog;
use crate::fmt::{LocaleFormat, NumberOptions};
use crate::locale::Locale;
use crate::SharedString;

/// The placeholder the unit messages put the formatted number at.
const PLACEHOLDER: &str = "{value}";

/// One unit that user-visible text can carry; the keys of `UNIT_MESSAGES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Unit {
    /// Length in millimetres.
    Mm,
    /// Area in square millimetres.
    Mm2,
    /// Volume in cubic millimetres.
    Mm3,
    /// Volume in cubic centimetres.
    Cm3,
    /// Length in micrometres (layer height).
    Um,
    /// Resin volume in millilitres.
    Ml,
    /// Millimetres per screen/layer pixel.
    MmPerPx,
    /// Weight in grams.
    G,
    /// File or memory size in bytes.
    B,
    /// File or memory size in kilobytes (1024 bytes).
    Kb,
    /// File or memory size in megabytes.
    Mb,
    /// File or memory size in gigabytes.
    Gb,
    /// File size in terabytes.
    Tb,
    /// Duration in seconds (exposure, run time).
    S,
    /// Duration in milliseconds.
    Ms,
    /// Frame rate, frames per second.
    Fps,
    /// A rate: how many times per second.
    PerSecond,
    /// Percentage.
    Percent,
    /// Angle in degrees.
    Degree,
    /// A multiplier, as in "1.50x" faster.
    Times,
    /// Compact count in thousands, as in "50K polygons".
    Thousands,
    /// Compact count in millions, as in "1.38M polygons".
    Millions,
}

impl Unit {
    /// Every unit, in `UNIT_MESSAGES` order.
    pub const ALL: [Unit; 22] = [
        Unit::Mm,
        Unit::Mm2,
        Unit::Mm3,
        Unit::Cm3,
        Unit::Um,
        Unit::Ml,
        Unit::MmPerPx,
        Unit::G,
        Unit::B,
        Unit::Kb,
        Unit::Mb,
        Unit::Gb,
        Unit::Tb,
        Unit::S,
        Unit::Ms,
        Unit::Fps,
        Unit::PerSecond,
        Unit::Percent,
        Unit::Degree,
        Unit::Times,
        Unit::Thousands,
        Unit::Millions,
    ];

    /// The TS key, e.g. `mm2`, `mmPerPx`, `perSecond`.
    pub const fn key(self) -> &'static str {
        match self {
            Unit::Mm => "mm",
            Unit::Mm2 => "mm2",
            Unit::Mm3 => "mm3",
            Unit::Cm3 => "cm3",
            Unit::Um => "um",
            Unit::Ml => "ml",
            Unit::MmPerPx => "mmPerPx",
            Unit::G => "g",
            Unit::B => "B",
            Unit::Kb => "KB",
            Unit::Mb => "MB",
            Unit::Gb => "GB",
            Unit::Tb => "TB",
            Unit::S => "s",
            Unit::Ms => "ms",
            Unit::Fps => "fps",
            Unit::PerSecond => "perSecond",
            Unit::Percent => "percent",
            Unit::Degree => "degree",
            Unit::Times => "times",
            Unit::Thousands => "thousands",
            Unit::Millions => "millions",
        }
    }

    /// The English message with its `{value}` placeholder — the msgid the
    /// catalogue is keyed on.
    pub const fn message(self) -> &'static str {
        match self {
            Unit::Mm => "{value} mm",
            Unit::Mm2 => "{value} mm²",
            Unit::Mm3 => "{value} mm³",
            Unit::Cm3 => "{value} cm³",
            Unit::Um => "{value} μm",
            Unit::Ml => "{value} ml",
            Unit::MmPerPx => "{value} mm/px",
            Unit::G => "{value} g",
            Unit::B => "{value} B",
            Unit::Kb => "{value} KB",
            Unit::Mb => "{value} MB",
            Unit::Gb => "{value} GB",
            Unit::Tb => "{value} TB",
            Unit::S => "{value} s",
            Unit::Ms => "{value} ms",
            Unit::Fps => "{value} fps",
            Unit::PerSecond => "{value} /s",
            Unit::Percent => "{value}%",
            Unit::Degree => "{value}°",
            Unit::Times => "{value}x",
            Unit::Thousands => "{value}K",
            Unit::Millions => "{value}M",
        }
    }

    /// The translator comment that ships with the message, so a regenerated
    /// catalogue keeps it.
    pub const fn comment(self) -> &'static str {
        match self {
            Unit::Mm => {
                "Length in millimetres. {value} is an already formatted number."
            }
            Unit::Mm2 => {
                "Area in square millimetres. {value} is an already formatted number."
            }
            Unit::Mm3 => {
                "Volume in cubic millimetres. {value} is an already formatted number."
            }
            Unit::Cm3 => {
                "Volume in cubic centimetres. {value} is an already formatted number."
            }
            Unit::Um => {
                "Length in micrometres (layer height). {value} is an already formatted number."
            }
            Unit::Ml => "Resin volume in millilitres. {value} is an already formatted number.",
            Unit::MmPerPx => {
                "Millimetres per screen/layer pixel. {value} is an already formatted number."
            }
            Unit::G => "Weight in grams. {value} is an already formatted number.",
            Unit::B => "File or memory size in bytes. {value} is an already formatted number.",
            Unit::Kb => {
                "File or memory size in kilobytes (1024 bytes). {value} is an already formatted number."
            }
            Unit::Mb => {
                "File or memory size in megabytes. {value} is an already formatted number."
            }
            Unit::Gb => {
                "File or memory size in gigabytes. {value} is an already formatted number."
            }
            Unit::Tb => {
                "File size in terabytes. {value} is an already formatted number."
            }
            Unit::S => {
                "Duration in seconds (exposure, run time). {value} is an already formatted number."
            }
            Unit::Ms => {
                "Duration in milliseconds. {value} is an already formatted number."
            }
            Unit::Fps => {
                "Frame rate, frames per second. {value} is an already formatted number."
            }
            Unit::PerSecond => {
                "A rate: how many times per second. {value} is an already formatted number."
            }
            Unit::Percent => "Percentage. {value} is an already formatted number.",
            Unit::Degree => {
                "Angle in degrees. {value} is an already formatted, possibly signed number."
            }
            Unit::Times => {
                "A multiplier, as in \"1.50x\" faster. {value} is an already formatted number."
            }
            Unit::Thousands => {
                "Compact count in thousands, as in \"50K polygons\". {value} is an already formatted number."
            }
            Unit::Millions => {
                "Compact count in millions, as in \"1.38M polygons\". {value} is an already formatted number."
            }
        }
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.key())
    }
}

/// The ported `UNIT_MESSAGES` table: every unit with its msgid, in
/// [`Unit::ALL`] order.
///
/// Built from [`Unit::ALL`] and [`Unit::message`] so the two can never drift;
/// [`Unit::comment`] carries the translator comment that ships with each row.
pub const UNIT_MESSAGES: [(Unit, &'static str); 22] = unit_messages();

const fn unit_messages() -> [(Unit, &'static str); 22] {
    let mut table = [(Unit::Mm, ""); 22];
    let mut index = 0;
    while index < Unit::ALL.len() {
        table[index] = (Unit::ALL[index], Unit::ALL[index].message());
        index += 1;
    }
    table
}

/// [`LocaleFormat`] plus the unit messages, bound to one locale and catalogue:
/// the Rust counterpart of `unitFormat(locale, translate)` and of the `fmt`
/// `useLocaleFormat()` hands a component.
#[derive(Clone, Debug)]
pub struct UnitFormat {
    numbers: LocaleFormat,
    catalog: Catalog,
}

impl UnitFormat {
    /// The formatters for `locale`, translating through that locale's shipped
    /// catalogue.
    pub fn for_locale(locale: &Locale) -> Self {
        Self::with_catalog(Catalog::for_locale(locale))
    }

    /// The formatters for `catalog`'s locale, translating through `catalog`.
    ///
    /// Use this when the catalogue differs from the shipped one — a plugin's
    /// messages, or a locale whose translation arrives later.
    pub fn with_catalog(catalog: Catalog) -> Self {
        Self { numbers: LocaleFormat::new(catalog.locale()), catalog }
    }

    /// The locale these formatters write for.
    pub fn locale(&self) -> &Locale {
        self.numbers.locale()
    }

    /// The catalogue unit messages are translated through.
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// `Intl.NumberFormat(locale).format(value)`.
    pub fn number(&self, value: f64) -> SharedString {
        self.numbers.number(value)
    }

    /// [`crate::fmt::number`] with explicit options.
    pub fn number_with(&self, value: f64, options: NumberOptions) -> SharedString {
        self.numbers.number_with(value, options)
    }

    /// [`crate::fmt::fixed`] for this locale.
    pub fn fixed(&self, value: f64, fraction_digits: usize) -> SharedString {
        self.numbers.fixed(value, fraction_digits)
    }

    /// [`crate::fmt::field`] for this locale.
    pub fn field(&self, value: f64, max_fraction_digits: usize) -> SharedString {
        self.numbers.field(value, max_fraction_digits)
    }

    /// [`crate::fmt::parse`] for this locale.
    pub fn parse(&self, text: &str) -> f64 {
        self.numbers.parse(text)
    }

    /// The decimal separator this locale writes.
    pub fn decimal_separator(&self) -> &'static str {
        self.numbers.decimal_separator()
    }

    /// `fixed(value, fraction_digits)` placed in the unit's message.
    pub fn quantity(&self, value: f64, fraction_digits: usize, unit: Unit) -> SharedString {
        self.unit(unit, &self.fixed(value, fraction_digits))
    }

    /// An already formatted number placed in the unit's message.
    ///
    /// The message comes from the catalogue, so the glyph and its spacing are
    /// the translator's: a Japanese catalogue can say `{value} ミリ` where the
    /// English msgid says `{value} mm`.
    pub fn unit(&self, unit: Unit, value: &str) -> SharedString {
        let message = self.catalog.text(unit.message());
        let Some((before, after)) = message.split_once(PLACEHOLDER) else {
            // A translation that dropped the placeholder cannot show the
            // number; show it as written rather than losing the unit.
            return SharedString::new_static(message);
        };

        let mut out = String::with_capacity(before.len() + value.len() + after.len());
        out.push_str(before);
        out.push_str(value);
        out.push_str(after);
        out.into()
    }
}

//! The behaviour the GPUI port has to keep from `useLocaleFormat()` and
//! `src/utils/unitFormatting.ts`.

use gpui_locale::fmt::{self, NumberOptions};
use gpui_locale::{Catalog, Locale, LocaleFormat, SharedString, Unit, UnitFormat, current, t};

fn locale(tag: &str) -> Locale {
    Locale::parse(tag)
}

/// The whole point of `fixed`: English and German show the same digits and the
/// same grouping, separated by different characters.
#[test]
fn fixed_differs_only_in_the_separator() {
    let english = LocaleFormat::new(&locale("en"));
    let german = LocaleFormat::new(&locale("de"));

    let en = english.fixed(1234.5, 2);
    let de = german.fixed(1234.5, 2);

    assert_eq!(&*en, "1,234.50");
    assert_eq!(&*de, "1.234,50");

    // Blank the separators and the two renders are identical.
    let skeleton = |text: &str| text.replace([',', '.'], "_");
    assert_eq!(skeleton(&en), skeleton(&de));

    // `cs` groups with a no-break space, not a dot.
    assert_eq!(&*LocaleFormat::new(&locale("cs")).fixed(1234.5, 2), "1\u{00a0}234,50");
    // `fr` uses a narrow no-break space.
    assert_eq!(
        &*LocaleFormat::new(&locale("fr")).fixed(1234567.89, 2),
        "1\u{202f}234\u{202f}567,89"
    );
}

/// `quantity` puts the number in the catalogue's message, so the unit glyph and
/// its spacing belong to the translation rather than to a hardcoded suffix.
#[test]
fn quantity_uses_the_catalogs_unit_text() {
    let english = UnitFormat::for_locale(&locale("en"));
    let german = UnitFormat::for_locale(&locale("de"));

    assert_eq!(&*english.quantity(1.0, 1, Unit::Mm), "1.0 mm");
    assert_eq!(&*german.quantity(1.0, 1, Unit::Mm), "1,0 mm");

    // A real translation that differs from the English msgid: Spanish puts a
    // space before the percent sign, so the catalog — not a suffix in code —
    // decides the text.
    let spanish = UnitFormat::for_locale(&locale("es"));
    assert_eq!(&*english.quantity(28.0, 0, Unit::Percent), "28%");
    assert_eq!(&*spanish.quantity(28.0, 0, Unit::Percent), "28 %");
    assert_eq!(&*spanish.quantity(12.345, 2, Unit::Mm2), "12,35 mm²");

    // A catalogue whose translation writes the unit in another script: the
    // number keeps German's separator, the unit comes from the catalogue.
    static CYRILLIC: &[(&str, &str)] = &[("{value} mm", "{value} мм")];
    let custom = UnitFormat::with_catalog(Catalog::new(locale("de"), CYRILLIC));
    assert_eq!(&*custom.quantity(1.0, 1, Unit::Mm), "1,0 мм");

    // Every unit message is round-tripped, placeholder and all.
    for unit in Unit::ALL {
        let en = english.unit(unit, "7");
        assert!(
            en.contains('7'),
            "the unit text must place the number: {unit:?} -> {en}"
        );
        assert!(!en.contains("{value}"), "{unit:?} left the placeholder: {en}");
    }

    // The ported `UNIT_MESSAGES` table is the same list, in the same order.
    assert_eq!(gpui_locale::units::UNIT_MESSAGES.len(), Unit::ALL.len());
    for (index, (unit, message)) in gpui_locale::units::UNIT_MESSAGES.iter().enumerate() {
        assert_eq!(*unit, Unit::ALL[index]);
        assert_eq!(*message, unit.message());
        assert!(!unit.comment().is_empty());
    }
}

/// A locale with no translation for a msgid shows the English text, which is
/// the msgid itself.
#[test]
fn catalog_falls_back_to_the_msgid() {
    let german = Catalog::for_locale(&locale("de"));
    assert_eq!(german.text("Add printer"), "Drucker hinzufügen");
    assert!(german.has("Add printer"));

    assert!(!german.has("No catalogue carries this line"));
    assert_eq!(german.text("No catalogue carries this line"), "No catalogue carries this line");

    // English has no table at all; everything falls back.
    let english = Catalog::english();
    assert!(english.is_empty());
    assert_eq!(english.text("Add printer"), "Add printer");
}

/// `t!` reads the thread's catalogue; the English default is the msgid.
#[test]
fn t_macro_translates_through_the_current_catalog() {
    gpui_locale::set_locale(&Locale::english());
    let label: SharedString = t!("Add printer");
    assert_eq!(&*label, "Add printer");

    current::set_locale(&locale("de"));
    assert_eq!(&*t!("Add printer"), "Drucker hinzufügen");
    assert_eq!(&*t!("No catalogue carries this line"), "No catalogue carries this line");

    // The explicit-catalogue form does not depend on thread state.
    let french = Catalog::for_locale(&locale("fr"));
    assert_eq!(&*t!(french, "Add printer"), "Ajouter une imprimante");

    current::set_locale(&Locale::english());
    assert_eq!(&*t!("Add printer"), "Add printer");
}

/// Numbers the TS `Intl.NumberFormat` path formats; these are the measured
/// `Intl` outputs, not `toFixed`'s.
#[test]
fn numbers_match_intl_rounding_and_grouping() {
    let english = LocaleFormat::new(&locale("en"));
    let spanish = LocaleFormat::new(&locale("es"));

    // Shortest-form rounding, ties away from zero.
    assert_eq!(&*english.fixed(1.005, 2), "1.01");
    assert_eq!(&*english.fixed(2.675, 2), "2.68");
    assert_eq!(&*english.fixed(1.25, 1), "1.3");
    assert_eq!(&*english.fixed(-2.5, 0), "-3");
    assert_eq!(&*english.fixed(0.0, 2), "0.00");
    assert_eq!(&*english.fixed(-0.0, 2), "-0.00");

    // Default options: up to three fraction digits, grouped.
    assert_eq!(&*english.number(1234.5), "1,234.5");
    assert_eq!(&*english.number(0.0001), "0");
    assert_eq!(&*english.number_with(1234.5, NumberOptions::fraction_digits(3)), "1,234.500");

    // Spanish only groups from five integer digits.
    assert_eq!(&*spanish.number(1000.0), "1000");
    assert_eq!(&*spanish.number(12345.0), "12.345");

    // Non-finite values, as `Intl` prints them.
    assert_eq!(&*english.number(f64::NAN), "NaN");
    assert_eq!(&*english.number(f64::INFINITY), "∞");
    assert_eq!(&*english.number(f64::NEG_INFINITY), "-∞");
}

/// `field` is what an editable field shows: no grouping, no trailing zeros, no
/// `-0`.
#[test]
fn field_and_parse_round_trip() {
    let german = LocaleFormat::new(&locale("de"));

    assert_eq!(&*german.field(2.5, 2), "2,5");
    assert_eq!(&*german.field(1234.5, 2), "1234,5");
    assert_eq!(&*german.field(-0.001, 2), "0");
    assert_eq!(&*german.field(2.675, 2), "2,68");

    assert_eq!(german.parse("2,5"), 2.5);
    assert_eq!(german.parse("2.5"), 2.5);
    assert_eq!(german.parse("  -1,25  "), -1.25);
    assert!(german.parse("1.234,5").is_nan());
    assert!(german.parse("").is_nan());

    // The free functions and the `LocaleFormat` object are the same path.
    assert_eq!(fmt::fixed(&locale("de"), 1234.5, 2), german.fixed(1234.5, 2));
    assert_eq!(fmt::decimal_separator(&locale("de")), ",");
}

#[test]
fn locales_canonicalise_and_carry_their_number_data() {
    let austrian = Locale::parse("de_AT");
    assert_eq!(austrian.tag(), "de-AT");
    assert_eq!(austrian.language(), "de");
    assert_eq!(austrian.region(), Some("AT"));
    assert_eq!(austrian.decimal_separator(), ",");
    assert_eq!(austrian.group_separator(), ".");
    assert_eq!(austrian.minimum_grouping_digits(), 1);
    assert!(austrian.is_supported());

    assert_eq!(Locale::parse("ZH_hant_tw").tag(), "zh-Hant-TW");
    assert_eq!(Locale::parse("").tag(), "en");
    assert_eq!(Locale::parse("!!").tag(), "en");
    assert!(!Locale::parse("ru").is_supported());
}

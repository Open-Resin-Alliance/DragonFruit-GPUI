//! Number, byte and timestamp formatting shared by the dialogs that print
//! measurements: the scene-file prompts, the printer mismatch sheet, the
//! diagnostics sheet, the history timeline and the slice-metrics sheet.

// ── Formatting helpers ──────────────────────────────────────────────────────

/// `fmt.fixed(value, digits)`.
pub(super) fn fixed(value: f64, digits: usize) -> String {
    format!("{value:.digits$}")
}

/// `fmt.quantity(value, digits, unit)` — `"{number} {unit}"`.
pub(super) fn quantity(value: f64, digits: usize, unit: &str) -> String {
    let number = fixed(value, digits);
    // `UNIT_MESSAGES` in `src/utils/unitFormatting.ts`: most units are
    // `'{value} {unit}'`, but `percent` has no space and `perSecond` is `'/s'`.
    match unit {
        "percent" => format!("{number}%"),
        "perSecond" => format!("{number} /s"),
        _ => format!("{number} {unit}"),
    }
}

/// `fmt.number(value)` — grouped integer.
pub(super) fn grouped(value: f64) -> String {
    let rounded = value.round();
    let negative = rounded < 0.0;
    let digits = format!("{}", rounded.abs() as u64);
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if negative {
        format!("-{out}")
    } else {
        out
    }
}

/// `value.toLocaleString()` for an optional number, or `—`.
pub(super) fn grouped_option(value: Option<f64>) -> String {
    match value.filter(|value| value.is_finite()) {
        Some(value) => grouped(value),
        None => "—".into(),
    }
}

/// A number without a trailing `.0` — the build-volume labels.
pub(super) fn num(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// `formatBytes` in `DiagnosticsModal` — `N/A`, `N B`, `N.N KB`, `N.NN MB`,
/// `N.NN GB`.
pub(super) fn format_bytes(bytes: Option<f64>) -> String {
    let Some(bytes) = bytes.filter(|bytes| bytes.is_finite()) else {
        return "N/A".into();
    };
    let abs = bytes.max(0.0);
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    if abs >= GB {
        quantity(abs / GB, 2, "GB")
    } else if abs >= MB {
        quantity(abs / MB, 2, "MB")
    } else if abs >= KB {
        quantity(abs / KB, 1, "KB")
    } else {
        quantity(abs, 0, "B")
    }
}

/// `formatPercent` in `DiagnosticsModal`.
pub(super) fn format_percent(value: Option<f64>) -> String {
    match value.filter(|value| value.is_finite()) {
        Some(value) => quantity(value * 100.0, 1, "percent"),
        None => "N/A".into(),
    }
}

/// `formatBytes` in `HistoryDebugModal` — `N B`, `N.N KB`, `N.N MB`.
pub(super) fn format_bytes_short(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// `formatHistoryDebugTimestamp` — `new Date(ts).toLocaleTimeString()`.
///
/// GPUI has no locale date formatting; the port prints the clock time of the
/// epoch-millisecond value in UTC as `HH:MM:SS` (24-hour).
pub(super) fn format_timestamp(ts: f64) -> String {
    if !ts.is_finite() {
        return format!("{ts}");
    }
    let total_seconds = (ts / 1000.0).floor() as i64;
    let seconds_of_day = total_seconds.rem_euclid(86_400);
    format!(
        "{:02}:{:02}:{:02}",
        seconds_of_day / 3600,
        (seconds_of_day % 3600) / 60,
        seconds_of_day % 60
    )
}

/// `formatHistoryLabel` from `src/history/formatHistoryLabel.ts`.
pub(super) fn format_history_label(value: &str) -> String {
    fn title_words(value: &str) -> String {
        value
            .split_whitespace()
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => {
                        format!("{}{}", first.to_uppercase(), chars.as_str().to_lowercase())
                    }
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn prettify_token(value: &str) -> String {
        // `([a-z0-9])([A-Z])` → `$1 $2`, then `_` and `-` become spaces.
        let mut spaced = String::with_capacity(value.len() + 4);
        let chars: Vec<char> = value.chars().collect();
        for (index, ch) in chars.iter().enumerate() {
            let previous_is_lower_or_digit = index > 0
                && (chars[index - 1].is_ascii_lowercase()
                    || chars[index - 1].is_ascii_digit());
            if index > 0 && ch.is_ascii_uppercase() && previous_is_lower_or_digit {
                spaced.push(' ');
            }
            spaced.push(*ch);
        }
        title_words(&spaced.replace('_', " ").replace('-', " "))
    }

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return value.to_string();
    }
    // `^([^:\s]+)\s*:\s*([^\s]+)(?:\s+(.+))?$`
    if let Some(colon) = trimmed.find(':') {
        let domain_raw = &trimmed[..colon];
        let rest = trimmed[colon + 1..].trim_start();
        if !domain_raw.is_empty() && !domain_raw.contains(char::is_whitespace) {
            let mut parts = rest.splitn(2, char::is_whitespace);
            let action_raw = parts.next().unwrap_or("");
            let suffix = parts.next().map(str::trim).filter(|s| !s.is_empty());
            if !action_raw.is_empty() {
                let domain = prettify_token(domain_raw);
                let action = prettify_token(action_raw);
                return match suffix {
                    Some(suffix) => format!("{domain}: {action} {suffix}"),
                    None => format!("{domain}: {action}"),
                };
            }
        }
    }
    prettify_token(trimmed)
}

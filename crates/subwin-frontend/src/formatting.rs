use std::fmt::Write;

/// Localized IEC units for byte quantities.
const UNITS: [&str; 7] = ["Б", "КиБ", "МиБ", "ГиБ", "ТиБ", "ПиБ", "ЭиБ"];

/// Localized IEC units for transfer speeds.
const SPEED_UNITS: [&str; 7] = ["Б/с", "КиБ/с", "МиБ/с", "ГиБ/с", "ТиБ/с", "ПиБ/с", "ЭиБ/с"];

/// Internal helper that formats a byte value using a custom array of unit
/// strings.
///
/// Scales the value by dividing by 1024 repeatedly until it falls below 1024,
/// then formats it with either exact bytes (for < 1024) or two decimal places.
///
/// This allows reuse for both size and speed formatting with different suffixes.
fn format_bytes_with_units(bytes: u64, units: [&str; 7]) -> String {
    let mut value = bytes as f64;
    let mut unit = 0;

    while value >= 1024.0 && unit < units.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{} {}", bytes, units[unit])
    } else {
        format!("{:.2} {}", value, units[unit])
    }
}

/// Formats a byte count into a human-readable localized string using IEC units.
pub fn format_bytes(bytes: u64) -> String {
    format_bytes_with_units(bytes, UNITS)
}

/// Formats a transfer rate (bytes per second) into a human-readable localized string.
pub fn format_speed(bytes_per_second: f64) -> String {
    format_bytes_with_units(bytes_per_second.round() as u64, SPEED_UNITS)
}

/// Formats an estimated time of arrival (ETA) or remaining duration in a
/// human-readable `HH:MM:SS` or `MM:SS` format.
pub fn format_eta(seconds: f64) -> String {
    let total = seconds.max(0.0).floor() as u64;

    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let secs = total % 60;

    let mut out = String::with_capacity(8);
    if hours > 0 {
        write!(&mut out, "{:02}:{:02}:{:02}", hours, minutes, secs).unwrap();
    } else {
        write!(&mut out, "{:02}:{:02}", minutes, secs).unwrap();
    }

    out
}

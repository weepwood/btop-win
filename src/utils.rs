use std::time::Duration;

pub fn format_bytes(value: u64) -> String {
    format_bytes_f64(value as f64)
}

pub fn format_rate(value: f64) -> String {
    format!("{}/s", format_bytes_f64(value.max(0.0)))
}

pub fn format_bytes_f64(value: f64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut size = value.max(0.0);
    let mut unit = 0;

    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{size:.0} {}", UNITS[unit])
    } else if size >= 100.0 {
        format!("{size:.0} {}", UNITS[unit])
    } else if size >= 10.0 {
        format!("{size:.1} {}", UNITS[unit])
    } else {
        format!("{size:.2} {}", UNITS[unit])
    }
}

pub fn format_uptime(seconds: u64) -> String {
    let duration = Duration::from_secs(seconds);
    let days = duration.as_secs() / 86_400;
    let hours = duration.as_secs() % 86_400 / 3_600;
    let minutes = duration.as_secs() % 3_600 / 60;

    if days > 0 {
        format!("{days}d {hours:02}h {minutes:02}m")
    } else if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else {
        format!("{minutes}m")
    }
}

pub fn percent(value: f64) -> u16 {
    value.clamp(0.0, 100.0).round() as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_binary_units() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1024), "1.00 KiB");
        assert_eq!(format_bytes(10 * 1024), "10.0 KiB");
    }

    #[test]
    fn formats_uptime() {
        assert_eq!(format_uptime(60), "1m");
        assert_eq!(format_uptime(3_660), "1h 01m");
    }
}

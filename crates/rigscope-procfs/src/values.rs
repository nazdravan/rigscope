//! Small single-value parsers.

/// Most logical CPUs rigscope will enumerate.
pub const MAX_CPUS: usize = 4096;

/// `"3400000\n"` (kHz, as in cpufreq) → `Some(3400)` MHz.
pub fn parse_khz(text: &str) -> Option<u32> {
    let khz: u64 = text.trim().parse().ok()?;
    u32::try_from(khz / 1000).ok()
}

/// Kernel CPU list (`"0-3,8,10-11"`) → sorted CPU numbers, at most [`MAX_CPUS`].
pub fn parse_cpu_list(text: &str) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    for part in text.trim().split(',') {
        if out.len() >= MAX_CPUS {
            break;
        }
        let (lo, hi) = match part.split_once('-') {
            Some((a, b)) => (a.trim().parse::<u32>(), b.trim().parse::<u32>()),
            None => (part.trim().parse::<u32>(), part.trim().parse::<u32>()),
        };
        let (Ok(lo), Ok(hi)) = (lo, hi) else { continue };
        let room = MAX_CPUS.saturating_sub(out.len());
        out.extend((lo..=hi).take(room));
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Cache size (`"32K"`, `"1024K"`, `"16M"`) → bytes.
pub fn parse_cache_size(text: &str) -> Option<u64> {
    let t = text.trim();
    let (num, mult) = match t.char_indices().last()? {
        (i, 'K') => (t.get(..i)?, 1024u64),
        (i, 'M') => (t.get(..i)?, 1024 * 1024),
        (i, 'G') => (t.get(..i)?, 1024 * 1024 * 1024),
        _ => (t, 1),
    };
    num.trim().parse::<u64>().ok()?.checked_mul(mult)
}

/// `MemTotal:` line of `/proc/meminfo` (kB) → bytes.
pub fn parse_meminfo_total(text: &str) -> Option<u64> {
    let line = text.lines().find(|l| l.starts_with("MemTotal:"))?;
    let kb: u64 = line
        .trim_start_matches("MemTotal:")
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    kb.checked_mul(1024)
}

/// `PRETTY_NAME` from `/etc/os-release`, quotes removed.
pub fn parse_os_release(text: &str) -> Option<String> {
    let value = text
        .lines()
        .find_map(|l| l.trim().strip_prefix("PRETTY_NAME="))?;
    let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
    rigscope_model::clean_text(value)
}

/// First number of `/proc/uptime` → whole seconds.
pub fn parse_uptime(text: &str) -> Option<u64> {
    let first = text.split_whitespace().next()?;
    let secs = first.split('.').next()?;
    secs.parse().ok()
}

/// A `/sys/class/dmi/id/*` value: trimmed, placeholders → `None`.
pub fn clean_dmi_value(text: &str) -> Option<String> {
    rigscope_model::clean_text(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn khz() {
        assert_eq!(parse_khz("3400000\n"), Some(3400));
        assert_eq!(parse_khz("799999"), Some(799));
        assert_eq!(parse_khz("abc"), None);
        assert_eq!(parse_khz(""), None);
    }

    #[test]
    fn cpu_list() {
        assert_eq!(parse_cpu_list("0-3,8,10-11\n"), vec![0, 1, 2, 3, 8, 10, 11]);
        assert_eq!(parse_cpu_list("5-2"), Vec::<u32>::new());
        assert_eq!(parse_cpu_list("7"), vec![7]);
        assert_eq!(parse_cpu_list(""), Vec::<u32>::new());
        assert_eq!(parse_cpu_list("0-4294967295").len(), MAX_CPUS);
    }

    #[test]
    fn cache_size() {
        assert_eq!(parse_cache_size("32K\n"), Some(32 * 1024));
        assert_eq!(parse_cache_size("16M"), Some(16 * 1024 * 1024));
        assert_eq!(parse_cache_size("512"), Some(512));
        assert_eq!(parse_cache_size("K"), None);
    }

    #[test]
    fn meminfo_total() {
        let t = "MemTotal:       16303428 kB\nMemFree:         1000 kB\n";
        assert_eq!(parse_meminfo_total(t), Some(16_303_428 * 1024));
        assert_eq!(parse_meminfo_total("MemFree: 1 kB"), None);
    }

    #[test]
    fn os_release() {
        let t = "NAME=\"Ubuntu\"\nPRETTY_NAME=\"Ubuntu 24.04.1 LTS\"\nID=ubuntu\n";
        assert_eq!(parse_os_release(t).as_deref(), Some("Ubuntu 24.04.1 LTS"));
        assert_eq!(
            parse_os_release("PRETTY_NAME=Arch Linux").as_deref(),
            Some("Arch Linux")
        );
        assert_eq!(parse_os_release("NAME=x"), None);
    }

    #[test]
    fn uptime() {
        assert_eq!(parse_uptime("12345.67 9999.00\n"), Some(12345));
        assert_eq!(parse_uptime("x"), None);
    }

    #[test]
    fn dmi_value() {
        assert_eq!(
            clean_dmi_value("ASUSTeK COMPUTER INC.\n").as_deref(),
            Some("ASUSTeK COMPUTER INC.")
        );
        assert_eq!(clean_dmi_value("Default string\n"), None);
    }
}

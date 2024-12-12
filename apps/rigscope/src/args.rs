//! Command-line arguments (hand-parsed; the set is tiny).

use anyhow::{bail, Result};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Args {
    /// Read a fixture directory instead of the real system.
    pub fixture: Option<String>,
    /// `en` or `ro`.
    pub lang: Option<String>,
    /// Initial section: `memory`, `cpu`, `system`.
    pub section: Option<String>,
}

impl Args {
    pub fn parse(mut it: impl Iterator<Item = String>) -> Result<Self> {
        let mut a = Args::default();
        while let Some(arg) = it.next() {
            let mut value = |name: &str| {
                it.next()
                    .ok_or_else(|| anyhow::anyhow!("{name} needs a value"))
            };
            match arg.as_str() {
                "--fixture" => a.fixture = Some(value("--fixture")?),
                "--lang" => a.lang = Some(value("--lang")?),
                "--section" => a.section = Some(value("--section")?),
                "-h" | "--help" => {
                    println!("usage: rigscope [--fixture DIR] [--lang en|ro] [--section memory|cpu|system]");
                    std::process::exit(0);
                }
                other => bail!("unknown argument: {other}"),
            }
        }
        Ok(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(v: &[&str]) -> Result<Args> {
        Args::parse(v.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_known_flags() {
        let a = parse(&["--fixture", "x", "--lang", "ro", "--section", "cpu"]).unwrap();
        assert_eq!(a.fixture.as_deref(), Some("x"));
        assert_eq!(a.lang.as_deref(), Some("ro"));
        assert_eq!(a.section.as_deref(), Some("cpu"));
    }

    #[test]
    fn rejects_unknown_and_missing_values() {
        assert!(parse(&["--bogus"]).is_err());
        assert!(parse(&["--fixture"]).is_err());
    }
}

//! Command-line arguments (hand-parsed; the set is tiny).

use anyhow::{Result, bail};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Args {
    /// Read a fixture directory instead of the real system.
    pub fixture: Option<String>,
    /// `en` or `ro`.
    pub lang: Option<String>,
    /// Initial section: `memory`, `cpu`, `system`.
    pub section: Option<String>,
    /// Port of the local control channel.
    pub control: Option<u16>,
    /// Save this machine (Windows) as a masked fixture into this directory, then exit.
    pub capture_fixture: Option<std::path::PathBuf>,
}

impl Args {
    /// Like [`Args::parse`], from raw OS strings: a non-UTF-8 argument is an error, not a panic.
    pub fn parse_os(it: impl Iterator<Item = std::ffi::OsString>) -> Result<Self> {
        let args: Vec<String> = it
            .map(|a| {
                a.into_string()
                    .map_err(|a| anyhow::anyhow!("argument is not valid UTF-8: {a:?}"))
            })
            .collect::<Result<_>>()?;
        Self::parse(args.into_iter())
    }

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
                "--capture-fixture" => {
                    a.capture_fixture = Some(value("--capture-fixture")?.into());
                }
                "--control" => {
                    let v = value("--control")?;
                    a.control = Some(
                        v.parse()
                            .map_err(|_| anyhow::anyhow!("--control: bad port {v}"))?,
                    );
                }
                "-h" | "--help" => {
                    println!(
                        "usage: rigscope [--fixture DIR] [--lang en|ro] [--section memory|cpu|system] [--control PORT] [--capture-fixture DIR]"
                    );
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
    fn capture_fixture_flag() {
        let a = parse(&["--capture-fixture", "out"]).unwrap();
        assert_eq!(
            a.capture_fixture.as_deref(),
            Some(std::path::Path::new("out"))
        );
        assert!(parse(&["--capture-fixture"]).is_err());
    }

    #[test]
    fn the_old_helper_flag_is_gone() {
        assert!(parse(&["--dump-dmi"]).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_argument_is_an_error_not_a_panic() {
        use std::os::unix::ffi::OsStringExt;
        let bad = std::ffi::OsString::from_vec(vec![b'-', b'-', 0xff]);
        assert!(Args::parse_os(vec![bad].into_iter()).is_err());
    }

    #[test]
    fn rejects_unknown_and_missing_values() {
        assert!(parse(&["--bogus"]).is_err());
        assert!(parse(&["--fixture"]).is_err());
    }
}

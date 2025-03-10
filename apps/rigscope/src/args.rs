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
    /// Port of the local control channel.
    pub control: Option<u16>,
    /// Privileged helper mode (run by pkexec). Must be the only argument.
    pub dump_dmi: bool,
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
                "--control" => {
                    let v = value("--control")?;
                    a.control = Some(
                        v.parse()
                            .map_err(|_| anyhow::anyhow!("--control: bad port {v}"))?,
                    );
                }
                "--dump-dmi" => a.dump_dmi = true,
                "-h" | "--help" => {
                    println!("usage: rigscope [--fixture DIR] [--lang en|ro] [--section memory|cpu|system] [--control PORT]");
                    std::process::exit(0);
                }
                other => bail!("unknown argument: {other}"),
            }
        }
        if a.dump_dmi
            && a != (Args {
                dump_dmi: true,
                ..Args::default()
            })
        {
            bail!("--dump-dmi takes no other arguments");
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
    fn dump_dmi_must_be_alone() {
        assert!(parse(&["--dump-dmi"]).unwrap().dump_dmi);
        assert!(parse(&["--dump-dmi", "--fixture", "x"]).is_err());
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

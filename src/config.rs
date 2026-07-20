use std::ffi::OsString;

use anyhow::{Result, bail};

pub const DEFAULT_INTERVAL_MS: u64 = 1_000;
pub const DEFAULT_HISTORY_POINTS: usize = 120;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub interval_ms: u64,
    pub history_points: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval_ms: DEFAULT_INTERVAL_MS,
            history_points: DEFAULT_HISTORY_POINTS,
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Self::parse_from(std::env::args_os().skip(1))
    }

    pub fn parse_from<I, S>(args: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        let mut config = Self::default();
        let mut args = args.into_iter().map(Into::into);

        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--interval" | "-i" => {
                    let value = next_value(&mut args, "--interval")?;
                    config.interval_ms = value
                        .parse::<u64>()
                        .map_err(|_| anyhow::anyhow!("invalid interval: {value}"))?;
                    if !(250..=5_000).contains(&config.interval_ms) {
                        bail!("interval must be between 250 and 5000 milliseconds");
                    }
                }
                "--history" => {
                    let value = next_value(&mut args, "--history")?;
                    config.history_points = value
                        .parse::<usize>()
                        .map_err(|_| anyhow::anyhow!("invalid history size: {value}"))?;
                    if !(30..=600).contains(&config.history_points) {
                        bail!("history must be between 30 and 600 points");
                    }
                }
                unknown => bail!("unknown argument: {unknown}\n\n{}", help_text()),
            }
        }

        Ok(config)
    }
}

fn next_value<I>(args: &mut I, option: &str) -> Result<String>
where
    I: Iterator<Item = OsString>,
{
    args.next()
        .map(|value| value.to_string_lossy().into_owned())
        .ok_or_else(|| anyhow::anyhow!("missing value for {option}"))
}

pub fn help_text() -> &'static str {
    "btop-win - Windows terminal system monitor\n\nUSAGE:\n    btop-win [OPTIONS]\n\nOPTIONS:\n    -i, --interval <MS>    Sampling interval, 250-5000 ms [default: 1000]\n        --history <COUNT>  History points, 30-600 [default: 120]\n    -h, --help             Print help\n    -V, --version          Print version\n\nKEYS:\n    q / Esc / Ctrl+C       Quit\n    p / Space              Pause or resume updates\n    s                      Cycle process sort column\n    Up/Down or j/k         Select a process\n    PageUp/PageDown        Move by ten processes\n    Home/End               Jump to first/last process\n    r                      Reset charts\n    ?                      Toggle help\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_custom_values() {
        let config = Config::parse_from(["--interval", "500", "--history", "240"]).unwrap();
        assert_eq!(config.interval_ms, 500);
        assert_eq!(config.history_points, 240);
    }

    #[test]
    fn rejects_too_fast_sampling() {
        assert!(Config::parse_from(["--interval", "100"]).is_err());
    }
}

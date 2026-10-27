use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalVideoBackend {
    Kitty,
    Sixel,
    TrueColor,
}

impl TerminalVideoBackend {
    pub fn from_environment() -> Result<Self, String> {
        let configured = env::var("ANI_CLI_TERMINAL_BACKEND").ok();
        let term_program = env::var("TERM_PROGRAM").ok();
        let term = env::var("TERM").ok();

        let detected = Self::detect(
            configured.as_deref(),
            term_program.as_deref(),
            term.as_deref(),
            env::var_os("KITTY_WINDOW_ID").is_some(),
            env::var_os("WEZTERM_PANE").is_some(),
            env::var_os("WT_SESSION").is_some(),
        )?;

        Ok(default_backend(
            detected,
            configured.as_deref(),
            cfg!(windows),
        ))
    }

    pub fn mpv_flags(self, available_outputs: &str) -> Result<Vec<String>, String> {
        self.mpv_flags_for_platform(available_outputs, cfg!(windows))
    }

    fn mpv_flags_for_platform(
        self,
        available_outputs: &str,
        is_windows: bool,
    ) -> Result<Vec<String>, String> {
        let preferred = match self {
            Self::Kitty => "kitty",
            Self::Sixel => "sixel",
            Self::TrueColor => "tct",
        };
        let selected = if output_driver_available(available_outputs, preferred) {
            preferred
        } else if !is_windows && output_driver_available(available_outputs, "tct") {
            "tct"
        } else {
            let platform_hint = if is_windows {
                " On Windows, automatic inline playback needs an MPV build with Sixel output for Windows Terminal; tct only works in ANSI-compatible terminals such as mintty."
            } else {
                ""
            };
            return Err(format!(
                "MPV does not provide a usable terminal video output driver (preferred: {preferred}); inspect `mpv --vo=help` and install a build with the required output driver.{platform_hint}"
            ));
        };

        Ok(vec![
            "--profile=sw-fast".to_owned(),
            format!("--vo={selected}"),
            "--force-window=no".to_owned(),
        ])
    }

    pub fn detect(
        configured: Option<&str>,
        term_program: Option<&str>,
        term: Option<&str>,
        has_kitty_window_id: bool,
        has_wezterm_pane: bool,
        has_windows_terminal_session: bool,
    ) -> Result<Self, String> {
        if let Some(value) = configured {
            match value.trim().to_ascii_lowercase().as_str() {
                "auto" => {}
                "kitty" => return Ok(Self::Kitty),
                "sixel" => return Ok(Self::Sixel),
                "tct" | "truecolor" | "true-color" => return Ok(Self::TrueColor),
                _ => {
                    return Err(format!(
                        "ANI_CLI_TERMINAL_BACKEND must be auto, kitty, sixel, or tct (received '{value}')"
                    ));
                }
            }
        }

        let term_program = term_program.unwrap_or_default().to_ascii_lowercase();
        let term = term.unwrap_or_default().to_ascii_lowercase();

        if has_kitty_window_id
            || has_wezterm_pane
            || ["kitty", "ghostty", "wezterm", "konsole"]
                .iter()
                .any(|name| term_program.contains(name) || term.contains(name))
        {
            return Ok(Self::Kitty);
        }

        if has_windows_terminal_session
            || [
                "windows_terminal",
                "windows terminal",
                "mlterm",
                "foot",
                "contour",
                "xterm-sixel",
            ]
            .iter()
            .any(|name| term_program.contains(name) || term.contains(name))
            || term.contains("sixel")
        {
            return Ok(Self::Sixel);
        }

        Ok(Self::TrueColor)
    }
}

fn default_backend(
    detected: TerminalVideoBackend,
    configured: Option<&str>,
    is_windows: bool,
) -> TerminalVideoBackend {
    let is_auto = configured.is_none_or(|value| value.eq_ignore_ascii_case("auto"));
    if is_windows && is_auto && detected == TerminalVideoBackend::TrueColor {
        TerminalVideoBackend::Sixel
    } else {
        detected
    }
}

fn output_driver_available(output: &str, driver: &str) -> bool {
    output.lines().any(|line| {
        line.split_whitespace()
            .next()
            .is_some_and(|name| name == driver)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_kitty_protocol_for_known_terminal_families() {
        assert_eq!(
            TerminalVideoBackend::detect(
                None,
                Some("Ghostty"),
                Some("xterm-ghostty"),
                false,
                false,
                false
            ),
            Ok(TerminalVideoBackend::Kitty)
        );
        assert_eq!(
            TerminalVideoBackend::detect(None, None, Some("xterm-kitty"), true, false, false),
            Ok(TerminalVideoBackend::Kitty)
        );
        assert_eq!(
            TerminalVideoBackend::detect(
                None,
                Some("WezTerm"),
                Some("xterm-256color"),
                false,
                true,
                false
            ),
            Ok(TerminalVideoBackend::Kitty)
        );
    }

    #[test]
    fn detects_sixel_for_known_terminal_families() {
        assert_eq!(
            TerminalVideoBackend::detect(None, None, Some("foot"), false, false, false),
            Ok(TerminalVideoBackend::Sixel)
        );
        assert_eq!(
            TerminalVideoBackend::detect(
                None,
                Some("Contour"),
                Some("contour"),
                false,
                false,
                false
            ),
            Ok(TerminalVideoBackend::Sixel)
        );
        assert_eq!(
            TerminalVideoBackend::detect(None, None, Some("xterm-sixel"), false, false, false),
            Ok(TerminalVideoBackend::Sixel)
        );
    }

    #[test]
    fn detects_windows_terminal_from_its_session_variable() {
        assert_eq!(
            TerminalVideoBackend::detect(
                None,
                Some("WindowsTerminal"),
                Some("xterm-256color"),
                false,
                false,
                true
            ),
            Ok(TerminalVideoBackend::Sixel)
        );
    }

    #[test]
    fn unknown_terminal_falls_back_to_true_color_cells() {
        assert_eq!(
            TerminalVideoBackend::detect(
                None,
                Some("generic-shell"),
                Some("xterm-256color"),
                false,
                false,
                false
            ),
            Ok(TerminalVideoBackend::TrueColor)
        );
    }

    #[test]
    fn configured_backend_overrides_terminal_detection() {
        assert_eq!(
            TerminalVideoBackend::detect(
                Some("SIXEL"),
                Some("Ghostty"),
                Some("xterm-ghostty"),
                true,
                true,
                false
            ),
            Ok(TerminalVideoBackend::Sixel)
        );
        assert_eq!(
            TerminalVideoBackend::detect(
                Some("tct"),
                Some("Kitty"),
                Some("xterm-kitty"),
                true,
                false,
                false
            ),
            Ok(TerminalVideoBackend::TrueColor)
        );
    }

    #[test]
    fn invalid_configured_backend_returns_a_clear_error() {
        let error =
            TerminalVideoBackend::detect(Some("not-a-backend"), None, None, false, false, false)
                .unwrap_err();
        assert!(error.contains("ANI_CLI_TERMINAL_BACKEND"));
    }

    #[test]
    fn windows_defaults_to_sixel_instead_of_nonportable_tct() {
        assert_eq!(
            default_backend(TerminalVideoBackend::TrueColor, None, true),
            TerminalVideoBackend::Sixel
        );
        assert_eq!(
            default_backend(TerminalVideoBackend::TrueColor, Some("auto"), true),
            TerminalVideoBackend::Sixel
        );
        assert_eq!(
            default_backend(TerminalVideoBackend::TrueColor, Some("tct"), true),
            TerminalVideoBackend::TrueColor
        );
        assert_eq!(
            default_backend(TerminalVideoBackend::TrueColor, None, false),
            TerminalVideoBackend::TrueColor
        );
        assert_eq!(
            default_backend(TerminalVideoBackend::Kitty, None, true),
            TerminalVideoBackend::Kitty
        );
    }

    #[test]
    fn windows_does_not_fall_back_to_tct_when_sixel_is_missing() {
        let error = TerminalVideoBackend::Sixel
            .mpv_flags_for_platform("Available video outputs:\n  gpu\n  tct\n", true)
            .unwrap_err();
        assert!(error.contains("does not provide a usable terminal video output driver"));
        assert!(error.contains("Sixel output for Windows Terminal"));

        let flags = TerminalVideoBackend::Sixel
            .mpv_flags_for_platform("Available video outputs:\n  gpu\n  sixel\n  tct\n", true)
            .unwrap();
        assert!(flags.contains(&"--vo=sixel".to_owned()));
    }

    #[test]
    fn selects_requested_graphics_driver_when_compiled_in() {
        let flags = TerminalVideoBackend::Kitty
            .mpv_flags("Available video outputs:\n  tct\n  kitty\n")
            .unwrap();
        assert!(flags.contains(&"--vo=kitty".to_owned()));
        assert!(flags.contains(&"--force-window=no".to_owned()));
        assert!(!flags.iter().any(|flag| flag == "--terminal=no"));
    }

    #[test]
    fn falls_back_to_tct_on_non_windows_only_when_mpv_lists_it() {
        let flags = TerminalVideoBackend::Sixel
            .mpv_flags_for_platform("Available video outputs:\n  gpu\n  tct\n", false)
            .unwrap();
        assert!(flags.contains(&"--vo=tct".to_owned()));

        let error = TerminalVideoBackend::Kitty
            .mpv_flags_for_platform("Available video outputs:\n  gpu\n  null\n", false)
            .unwrap_err();
        assert!(error.contains("usable terminal video output driver"));
    }
}

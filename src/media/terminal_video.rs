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
    line
      .split_whitespace()
      .next()
      .is_some_and(|name| name == driver)
  })
}

#[cfg(test)]
#[path = "../../tests/unit/media/terminal_video.rs"]
mod tests;

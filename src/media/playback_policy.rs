//! Deterministic playback target selection shared by platform integrations.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackTarget {
  Inline,
  External,
}

/// Select inline playback when explicitly requested and supported; otherwise
/// fall back to external playback if it is available.
pub fn choose_target(
  inline_requested: bool,
  inline_available: bool,
  external_available: bool,
) -> Result<PlaybackTarget, &'static str> {
  if inline_requested && inline_available {
    return Ok(PlaybackTarget::Inline);
  }

  if external_available {
    return Ok(PlaybackTarget::External);
  }

  Err("no supported playback target is available")
}

#[cfg(test)]
#[path = "../../tests/unit/media/playback_policy.rs"]
mod tests;

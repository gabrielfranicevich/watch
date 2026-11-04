#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LanguagePair {
  pub audio: Option<String>,
  pub subtitles: Option<String>,
}

impl LanguagePair {
  pub fn new(audio: Option<&str>, subtitles: Option<&str>) -> Self {
    Self {
      audio: normalize_code(audio),
      subtitles: normalize_code(subtitles),
    }
  }

  pub fn known(audio: &str, subtitles: &str) -> Self {
    Self::new(Some(audio), Some(subtitles))
  }

  pub fn label(&self) -> String {
    format!(
      "{}/{}",
      self.audio.as_deref().unwrap_or("?"),
      self.subtitles.as_deref().unwrap_or("?")
    )
  }
}

fn normalize_code(code: Option<&str>) -> Option<String> {
  code
    .map(str::trim)
    .filter(|code| !code.is_empty())
    .map(str::to_ascii_lowercase)
}

#[cfg(test)]
#[path = "../../tests/unit/domain/language.rs"]
mod tests;

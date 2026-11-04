use super::language::LanguagePair;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anime {
  pub id: String,
  pub title: String,
  pub url: String,
  pub language_pair: Option<LanguagePair>,
}

impl Anime {
  pub fn new(id: impl Into<String>, title: impl Into<String>, url: impl Into<String>) -> Self {
    Self {
      id: id.into(),
      title: title.into(),
      url: url.into(),
      language_pair: None,
    }
  }
}

#[cfg(test)]
#[path = "../../tests/unit/domain/anime.rs"]
mod tests;

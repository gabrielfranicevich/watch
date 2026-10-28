#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anime {
  pub id: String,
  pub title: String,
  pub url: String,
}

impl Anime {
  pub fn new(id: impl Into<String>, title: impl Into<String>, url: impl Into<String>) -> Self {
    Self {
      id: id.into(),
      title: title.into(),
      url: url.into(),
    }
  }
}

#[cfg(test)]
#[path = "../../tests/unit/domain/anime.rs"]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
  pub episode_number: String,
  pub anime_id: String,
  pub anime_title: String,
  pub provider: Option<String>,
}

impl HistoryEntry {
  pub fn new(
    episode_number: impl Into<String>,
    anime_id: impl Into<String>,
    anime_title: impl Into<String>,
  ) -> Self {
    Self {
      episode_number: episode_number.into(),
      anime_id: anime_id.into(),
      anime_title: anime_title.into(),
      provider: None,
    }
  }

  pub fn with_provider(
    episode_number: impl Into<String>,
    anime_id: impl Into<String>,
    anime_title: impl Into<String>,
    provider: impl Into<String>,
  ) -> Self {
    Self {
      episode_number: episode_number.into(),
      anime_id: anime_id.into(),
      anime_title: anime_title.into(),
      provider: Some(provider.into()),
    }
  }
}

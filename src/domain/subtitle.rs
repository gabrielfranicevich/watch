#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subtitle {
    pub url: String,
    pub language: Option<String>,
}

impl Subtitle {
    pub fn new(url: impl Into<String>, language: Option<String>) -> Self {
        Self {
            url: url.into(),
            language,
        }
    }
}

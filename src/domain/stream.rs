#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream {
    pub url: String,
    pub quality: Option<String>,
    pub referer: Option<String>,
}

impl Stream {
    pub fn new(url: impl Into<String>, quality: Option<String>, referer: Option<String>) -> Self {
        Self {
            url: url.into(),
            quality,
            referer,
        }
    }
}

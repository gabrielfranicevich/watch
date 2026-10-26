#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Episode {
    pub number: String,
    pub title: Option<String>,
    pub url: String,
}

impl Episode {
    pub fn new(number: impl Into<String>, title: Option<String>, url: impl Into<String>) -> Self {
        Self {
            number: number.into(),
            title,
            url: url.into(),
        }
    }
}

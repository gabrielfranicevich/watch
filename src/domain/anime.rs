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
mod tests {
    use super::*;

    #[test]
    fn stores_provider_identity_and_title() {
        let anime = Anime::new("frieren", "Frieren", "https://example.test/anime/frieren");
        assert_eq!(anime.id, "frieren");
        assert_eq!(anime.title, "Frieren");
    }
}

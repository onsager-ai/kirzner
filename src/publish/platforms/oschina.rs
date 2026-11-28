use super::{Article, PlatformArticle, PublisherAdapter, Result, normalize};

const MAX_OSCHINA_TAGS: usize = 5;

pub(super) struct Oschina;

impl PublisherAdapter for Oschina {
    fn id(&self) -> &str {
        "oschina"
    }

    fn creator_url(&self) -> Option<&str> {
        Some("https://my.oschina.net/")
    }

    fn normalize(&self, article: &Article) -> Result<PlatformArticle> {
        let mut normalized = normalize(article, "开源中国", Some(MAX_OSCHINA_TAGS))?;
        normalized.category = None;
        Ok(normalized)
    }

    fn checklist_steps(&self) -> &[&str] {
        &[
            "Confirm the summary and tags in 开源中国. Tag availability may vary.",
            "Select the declared cover if provided.",
        ]
    }
}

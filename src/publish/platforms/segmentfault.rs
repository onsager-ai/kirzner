use super::{Article, PlatformArticle, PublisherAdapter, Result, normalize};

const MAX_SEGMENTFAULT_TAGS: usize = 5;

pub(super) struct Segmentfault;

impl PublisherAdapter for Segmentfault {
    fn id(&self) -> &str {
        "segmentfault"
    }

    fn creator_url(&self) -> Option<&str> {
        Some("https://segmentfault.com/write")
    }

    fn normalize(&self, article: &Article) -> Result<PlatformArticle> {
        let mut normalized = normalize(article, "SegmentFault", Some(MAX_SEGMENTFAULT_TAGS))?;
        normalized.category = None;
        Ok(normalized)
    }

    fn checklist_steps(&self) -> &[&str] {
        &[
            "Confirm the summary and tags in 思否. Tag availability may vary.",
            "Select the declared cover if provided.",
        ]
    }
}

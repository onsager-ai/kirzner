use super::{Article, PlatformArticle, PublisherAdapter, Result, normalize_with_title_limit};

const MAX_V2EX_TITLE_CHARS: usize = 120;

pub(super) struct V2ex;

impl PublisherAdapter for V2ex {
    fn id(&self) -> &str {
        "v2ex"
    }

    fn creator_url(&self) -> Option<&str> {
        Some("https://www.v2ex.com/write")
    }

    fn normalize(&self, article: &Article) -> Result<PlatformArticle> {
        let mut normalized =
            normalize_with_title_limit(article, "V2EX", MAX_V2EX_TITLE_CHARS, None)?;
        normalized.summary = None;
        normalized.tags.clear();
        Ok(normalized)
    }

    fn checklist_steps(&self) -> &[&str] {
        &[
            "Choose a target node from the category field. V2EX allows one node per launch topic, such as 分享创造.",
            "Confirm the title and node in V2EX before submitting.",
        ]
    }
}

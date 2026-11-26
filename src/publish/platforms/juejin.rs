use super::{Article, PlatformArticle, PublisherAdapter, Result, normalize};

const MAX_JUEJIN_TAGS: usize = 5;

pub(super) struct Juejin;

impl PublisherAdapter for Juejin {
    fn id(&self) -> &str {
        "juejin"
    }

    fn creator_url(&self) -> Option<&str> {
        Some("https://juejin.cn/editor/drafts/new")
    }

    fn normalize(&self, article: &Article) -> Result<PlatformArticle> {
        normalize(article, "Juejin", Some(MAX_JUEJIN_TAGS))
    }

    fn checklist_steps(&self) -> &[&str] {
        &[
            "Verify the category and tags in Juejin’s creator UI. Availability may vary.",
            "Select the declared cover if provided.",
        ]
    }
}

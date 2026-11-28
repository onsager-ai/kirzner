use super::{Article, PlatformArticle, PublisherAdapter, Result, normalize};

const MAX_CNBLOGS_TAGS: usize = 10;

pub(super) struct Cnblogs;

impl PublisherAdapter for Cnblogs {
    fn id(&self) -> &str {
        "cnblogs"
    }

    fn creator_url(&self) -> Option<&str> {
        Some("https://i.cnblogs.com/posts/edit")
    }

    fn normalize(&self, article: &Article) -> Result<PlatformArticle> {
        normalize(article, "博客园", Some(MAX_CNBLOGS_TAGS))
    }

    fn checklist_steps(&self) -> &[&str] {
        &[
            "Confirm the summary, category, tags, and cover in 博客园. Category and tag availability may vary.",
        ]
    }
}

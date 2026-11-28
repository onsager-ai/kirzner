use super::{Article, PlatformArticle, PublisherAdapter, Result, normalize};

pub(super) struct Jianshu;

impl PublisherAdapter for Jianshu {
    fn id(&self) -> &str {
        "jianshu"
    }

    fn creator_url(&self) -> Option<&str> {
        Some("https://www.jianshu.com/writer")
    }

    fn normalize(&self, article: &Article) -> Result<PlatformArticle> {
        let mut normalized = normalize(article, "简书", None)?;
        normalized.tags.clear();
        normalized.category = None;
        Ok(normalized)
    }

    fn checklist_steps(&self) -> &[&str] {
        &[
            "Confirm the summary and cover in 简书 before publishing.",
            "Check the rendered Markdown in the editor.",
        ]
    }
}

use super::{Article, PlatformArticle, PublisherAdapter, Result, normalize};

pub(super) struct Zhihu;

impl PublisherAdapter for Zhihu {
    fn id(&self) -> &str {
        "zhihu"
    }

    fn creator_url(&self) -> Option<&str> {
        Some("https://zhuanlan.zhihu.com/write")
    }

    fn normalize(&self, article: &Article) -> Result<PlatformArticle> {
        let mut normalized = normalize(article, "Zhihu", None)?;
        normalized.category = None;
        Ok(normalized)
    }

    fn tags_are_topic_suggestions(&self) -> bool {
        true
    }

    fn checklist_steps(&self) -> &[&str] {
        &[
            "Verify that code blocks render with the original indentation and language tags.",
            "Choose the declared cover if provided.",
            "Choose and attach matching topics manually in Zhihu’s UI. Availability may vary.",
        ]
    }
}

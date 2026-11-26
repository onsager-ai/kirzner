use super::{Article, PlatformArticle, PublisherAdapter, Result, normalize};

const MAX_CSDN_TAGS: usize = 10;

pub(super) struct Csdn;

impl PublisherAdapter for Csdn {
    fn id(&self) -> &str {
        "csdn"
    }

    fn creator_url(&self) -> Option<&str> {
        Some("https://editor.csdn.net/md/")
    }

    fn normalize(&self, article: &Article) -> Result<PlatformArticle> {
        normalize(article, "CSDN", Some(MAX_CSDN_TAGS))
    }

    fn checklist_steps(&self) -> &[&str] {
        &[
            "Use the CSDN Markdown editor to import or paste the article.",
            "Confirm category, tags, and cover in CSDN. Category and tag availability may vary.",
        ]
    }
}

mod cnblogs;
mod csdn;
mod jianshu;
mod juejin;
mod oschina;
mod segmentfault;
mod v2ex;
mod zhihu;

use super::{Article, GenericAdapter, PlatformArticle, PublisherAdapter, Result, invalid};
use cnblogs::Cnblogs;
use csdn::Csdn;
use jianshu::Jianshu;
use juejin::Juejin;
use oschina::Oschina;
use segmentfault::Segmentfault;
use v2ex::V2ex;
use zhihu::Zhihu;

const MAX_TITLE_CHARS: usize = 100;
const ADAPTERS: [&(dyn PublisherAdapter + Sync); 9] = [
    &GenericAdapter,
    &Juejin,
    &Zhihu,
    &Csdn,
    &Segmentfault,
    &Cnblogs,
    &V2ex,
    &Jianshu,
    &Oschina,
];

pub(super) fn lookup(platform: &str) -> Result<&'static (dyn PublisherAdapter + Sync)> {
    ADAPTERS
        .into_iter()
        .find(|adapter| adapter.id() == platform)
        .ok_or_else(|| {
            let supported = ADAPTERS.map(|adapter| adapter.id()).join(", ");
            invalid(format!(
                "Unknown publication platform {platform:?}. Supported platforms: {supported}"
            ))
        })
}

fn normalize(article: &Article, name: &str, max_tags: Option<usize>) -> Result<PlatformArticle> {
    normalize_with_title_limit(article, name, MAX_TITLE_CHARS, max_tags)
}

fn normalize_with_title_limit(
    article: &Article,
    name: &str,
    max_title_chars: usize,
    max_tags: Option<usize>,
) -> Result<PlatformArticle> {
    let metadata = article.metadata();
    if metadata.title.trim().is_empty() {
        return Err(invalid(format!("{name} requires a non-empty title")));
    }
    if metadata.title.chars().count() > max_title_chars {
        return Err(invalid(format!(
            "{name} titles may have at most {max_title_chars} Unicode characters"
        )));
    }
    if let Some(limit) = max_tags.filter(|&limit| metadata.tags.len() > limit) {
        return Err(invalid(format!("{name} allows at most {limit} tags")));
    }
    let mut normalized = GenericAdapter.normalize(article)?;
    let trimmed_len = normalized.markdown.trim_end().len();
    normalized.markdown.truncate(trimmed_len);
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn article(title: &str, tags: usize) -> Article {
        let tags: Vec<_> = (0..tags).map(|index| format!("tag-{index}")).collect();
        let tags = tags.join(", ");
        let input = format!("---\ntitle: {title}\ntags: [{tags}]\n---\nBody  \n\n");
        Article::parse(input.into_bytes(), |_, _| unreachable!()).unwrap()
    }

    #[test]
    fn validates_unicode_title_limits_and_trims_document_end() {
        assert_eq!(MAX_TITLE_CHARS, 100);
        for id in [
            "juejin",
            "zhihu",
            "csdn",
            "segmentfault",
            "cnblogs",
            "v2ex",
            "jianshu",
            "oschina",
        ] {
            let adapter = lookup(id).unwrap();
            let limit = if id == "v2ex" { 120 } else { 100 };
            let title = "知".repeat(limit);
            let normalized = adapter.normalize(&article(&title, 0)).unwrap();
            assert_eq!(normalized.title, title);
            assert_eq!(normalized.markdown, "Body");
            let too_long = article(&"知".repeat(limit + 1), 0);
            let error = adapter.normalize(&too_long).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains(&format!("{limit} Unicode characters"))
            );
            let mut invalid = article("Title", 0);
            for title in ["", " \u{3000}\t"] {
                invalid.record.title = title.into();
                let error = adapter.normalize(&invalid).unwrap_err();
                assert!(error.to_string().contains("non-empty title"));
            }
        }
    }

    #[test]
    fn pins_platform_ids_creator_urls_and_tag_limits() {
        for (id, creator, max_tags) in [
            ("juejin", "https://juejin.cn/editor/drafts/new", 5),
            ("csdn", "https://editor.csdn.net/md/", 10),
            ("segmentfault", "https://segmentfault.com/write", 5),
            ("cnblogs", "https://i.cnblogs.com/posts/edit", 10),
            ("oschina", "https://my.oschina.net/", 5),
        ] {
            let adapter = lookup(id).unwrap();
            assert_eq!(adapter.id(), id);
            assert_eq!(adapter.creator_url(), Some(creator));
            let valid = article("Title", max_tags);
            let normalized = adapter.normalize(&valid).unwrap();
            assert_eq!(normalized.tags, valid.record.tags);
            let too_many = article("Title", max_tags + 1);
            let error = adapter.normalize(&too_many).unwrap_err();
            assert!(error.to_string().contains(&format!("{max_tags} tags")));
        }
        let zhihu = lookup("zhihu").unwrap();
        assert_eq!(zhihu.id(), "zhihu");
        assert_eq!(
            zhihu.creator_url(),
            Some("https://zhuanlan.zhihu.com/write")
        );
        assert!(zhihu.tags_are_topic_suggestions());
        let valid = article("Title", 11);
        assert_eq!(zhihu.normalize(&valid).unwrap().tags, valid.record.tags);

        for (id, creator) in [
            ("v2ex", "https://www.v2ex.com/write"),
            ("jianshu", "https://www.jianshu.com/writer"),
        ] {
            let adapter = lookup(id).unwrap();
            assert_eq!(adapter.creator_url(), Some(creator));
        }
        let v2ex = lookup("v2ex").unwrap();
        let mut article = article("Title", 1);
        article.record.category = Some("分享创造".into());
        let normalized = v2ex.normalize(&article).unwrap();
        assert_eq!(normalized.category.as_deref(), Some("分享创造"));
        assert!(
            v2ex.checklist_steps()
                .iter()
                .any(|step| step.contains("one node"))
        );
    }
}

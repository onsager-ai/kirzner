// Portions ported from ArtiPub, Copyright (c) 2019 Crawlab Team.
// Licensed under the BSD-3-Clause License; see THIRD_PARTY_NOTICES.md.
use super::bounded_body;
use reqwest::{Client, StatusCode, header};
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::{sync::LazyLock, time::Duration};
use url::{Host, Url};

const HTML_LIMIT: usize = 512 * 1024;
const IMAGE_LIMIT: usize = 5 * 1024 * 1024;
const THUMB_LIMIT: usize = 1_000_000;
pub(super) const TIMEOUT: Duration = Duration::from_secs(10);

static META: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("meta").expect("static selector"));
static TITLE: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("title").expect("static selector"));

pub(super) fn eligible(url: &Url) -> bool {
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    match url.host() {
        Some(Host::Domain(host)) => {
            let host = host.trim_end_matches('.');
            host != "localhost" && !host.ends_with(".localhost")
        }
        Some(Host::Ipv4(ip)) => {
            !(ip.is_loopback() || ip.is_private() || ip.is_link_local() || ip.octets()[0] == 0)
        }
        Some(Host::Ipv6(ip)) => {
            !(ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
                || ip.to_ipv4_mapped().is_some_and(|v4| {
                    v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.octets()[0] == 0
                }))
        }
        None => false,
    }
}

struct Metadata {
    title: String,
    description: String,
    image: Option<String>,
}

fn metadata(html: &[u8]) -> Metadata {
    let html = Html::parse_document(&String::from_utf8_lossy(html));
    let mut title = None;
    let mut og_description = None;
    let mut description = None;
    let mut image = None;
    for meta in html.select(&META) {
        let attrs = meta.value();
        let key = attrs
            .attr("property")
            .or_else(|| attrs.attr("name"))
            .unwrap_or("");
        let Some(content) = attrs.attr("content").filter(|content| !content.is_empty()) else {
            continue;
        };
        let value = match key.to_ascii_lowercase().as_str() {
            "og:title" => &mut title,
            "og:description" => &mut og_description,
            "description" => &mut description,
            "og:image" => &mut image,
            _ => continue,
        };
        if value.is_none() {
            *value = Some(content.to_owned());
        }
    }
    let title = title.unwrap_or_else(|| {
        html.select(&TITLE)
            .next()
            .map(|node| node.text().collect::<String>().trim().to_owned())
            .unwrap_or_default()
    });
    Metadata {
        title: title.chars().take(300).collect(),
        description: og_description
            .or(description)
            .unwrap_or_default()
            .chars()
            .take(1000)
            .collect(),
        image,
    }
}

pub(super) struct Card {
    pub(super) embed: Value,
    pub(super) thumb: Option<(Vec<u8>, String)>,
}

fn get(client: &Client, url: Url) -> reqwest::RequestBuilder {
    client
        .get(url)
        .timeout(TIMEOUT)
        .header(header::USER_AGENT, "kirzner-card-fetch")
}

fn mime(response: &reqwest::Response) -> String {
    response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

async fn page(client: &Client, mut url: Url) -> Option<(Url, Metadata)> {
    for redirects in 0..=3 {
        let response = get(client, url.clone()).send().await.ok()?;
        match response.status() {
            StatusCode::MOVED_PERMANENTLY
            | StatusCode::FOUND
            | StatusCode::SEE_OTHER
            | StatusCode::TEMPORARY_REDIRECT
            | StatusCode::PERMANENT_REDIRECT => {
                if redirects == 3 {
                    return None;
                }
                url = url
                    .join(response.headers().get(header::LOCATION)?.to_str().ok()?)
                    .ok()?;
                if !eligible(&url) {
                    return None;
                }
            }
            status if status.is_success() && mime(&response) == "text/html" => {
                let body = bounded_body(response, HTML_LIMIT).await.ok()?;
                return Some((url, metadata(&body)));
            }
            _ => return None,
        }
    }
    None
}

pub(super) async fn fetch(client: &Client, url: Url) -> Option<Card> {
    let (page, metadata) = page(client, url).await?;
    if metadata.title.is_empty() && metadata.description.is_empty() {
        return None;
    }
    let mut thumb = None;
    if let Some(image) = metadata.image {
        let url = page.join(&image).ok()?;
        if eligible(&url) {
            let response = get(client, url).send().await.ok()?;
            let mime = mime(&response);
            if response.status().is_success()
                && matches!(mime.as_str(), "image/jpeg" | "image/png" | "image/webp")
            {
                let bytes = bounded_body(response, IMAGE_LIMIT).await.ok()?;
                if bytes.len() <= THUMB_LIMIT {
                    thumb = Some((bytes, mime));
                }
            }
        }
    }
    Some(Card {
        embed: json!({
            "$type": "app.bsky.embed.external",
            "external": {
                "uri": page.as_str(),
                "title": metadata.title,
                "description": metadata.description,
            }
        }),
        thumb,
    })
}

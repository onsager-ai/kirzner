use super::{PublishError, Result, invalid, sha256};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArticleRecord {
    pub id: String,
    pub title: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub category: Option<String>,
    pub cover: Option<String>,
    pub source_sha256: String,
    pub normalized_body_sha256: String,
    pub assets: Vec<Asset>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub path: String,
    pub media_type: String,
    pub bytes: usize,
    pub order: usize,
    pub sha256: String,
}

pub struct Article {
    pub(super) record: ArticleRecord,
    pub(super) markdown: String,
    pub(super) source: Vec<u8>,
    pub(super) asset_bytes: Vec<Vec<u8>>,
}

impl Article {
    /// Import UTF-8 Markdown with `---` frontmatter and same-line `![alt](path)` images.
    /// Frontmatter supports scalar fields and scalar, inline-list, or block-list tags.
    /// The cover is read first, followed by inline destinations in body order.
    /// Use angle brackets or percent encoding for spaces in inline paths.
    pub fn read(path: &Path) -> Result<Self> {
        let base = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .canonicalize()?;
        Self::parse(fs::read(path)?, |relative, _| {
            let target =
                base.join(relative)
                    .canonicalize()
                    .map_err(|source| PublishError::Image {
                        path: relative.display().to_string(),
                        source,
                    })?;
            if !target.starts_with(&base) || !target.is_file() {
                return Err(invalid(format!(
                    "Image must be a file inside the article directory: {}",
                    relative.display()
                )));
            }
            Ok(fs::read(target)?)
        })
    }

    pub fn metadata(&self) -> &ArticleRecord {
        &self.record
    }

    pub fn markdown(&self) -> &str {
        &self.markdown
    }

    pub(super) fn parse(
        source: Vec<u8>,
        mut read_image: impl FnMut(&Path, usize) -> Result<Vec<u8>>,
    ) -> Result<Self> {
        let normalized = std::str::from_utf8(&source)
            .map_err(|_| invalid("Article must be UTF-8 Markdown"))?
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        let (fields, body) = frontmatter(&normalized)?;
        let mut assets = Vec::<Asset>::new();
        let mut asset_bytes = Vec::new();
        let mut paths = Vec::<PathBuf>::new();
        let mut asset_path = |destination: &str| -> Result<String> {
            let decoded = local_path(destination)?;
            let path = Path::new(&decoded);
            if let Some(index) = paths.iter().position(|seen| seen.as_path() == path) {
                return Ok(assets[index].path.clone());
            }
            let (extension, media_type) = image_type(path)?;
            let order = assets.len() + 1;
            let bytes = read_image(path, order - 1)?;
            let stem = safe_stem(path);
            let output = format!("assets/{order:03}-{stem}.{extension}");
            assets.push(Asset {
                path: output.clone(),
                media_type: media_type.into(),
                bytes: bytes.len(),
                order,
                sha256: sha256(&bytes),
            });
            asset_bytes.push(bytes);
            paths.push(path.to_path_buf());
            Ok(output)
        };
        let cover = fields.cover.as_deref().map(&mut asset_path).transpose()?;
        let markdown = rewrite_images(body, &mut asset_path)?;
        let source_sha256 = sha256(&source);
        let mut identity = Sha256::new();
        identity.update(source_sha256.as_bytes());
        for asset in &assets {
            identity.update(asset.sha256.as_bytes());
        }
        let record = ArticleRecord {
            id: format!("article-{:x}", identity.finalize()),
            title: fields.title,
            summary: fields.summary,
            tags: fields.tags,
            category: fields.category,
            cover,
            source_sha256,
            normalized_body_sha256: sha256(markdown.as_bytes()),
            assets,
        };
        Ok(Self {
            record,
            markdown,
            source,
            asset_bytes,
        })
    }
}

#[derive(Default)]
struct Frontmatter {
    title: String,
    summary: Option<String>,
    tags: Vec<String>,
    category: Option<String>,
    cover: Option<String>,
}

fn frontmatter(markdown: &str) -> Result<(Frontmatter, &str)> {
    let mut body = markdown
        .trim_start_matches('\u{feff}')
        .strip_prefix("---\n")
        .ok_or_else(|| invalid("Article must start with --- frontmatter"))?;
    let mut fields = Frontmatter::default();
    let mut keys = Vec::new();
    let mut list = false;
    while !body.is_empty() {
        let (line, rest) = body.split_once('\n').unwrap_or((body, ""));
        body = rest;
        if line == "---" {
            if fields.title.trim().is_empty() {
                return Err(invalid("Article frontmatter requires a title"));
            }
            return Ok((fields, body));
        }
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if list && let Some(tag) = line.strip_prefix("- ") {
            push_tag(&mut fields.tags, tag)?;
            continue;
        }
        let (key, value) = line
            .split_once(':')
            .ok_or_else(|| invalid(format!("Invalid frontmatter line: {line}")))?;
        let key = key.trim();
        let value = value.trim();
        if keys.contains(&key) {
            return Err(invalid(format!("Duplicate frontmatter field: {key}")));
        }
        keys.push(key);
        list = key == "tags" && value.is_empty();
        match key {
            "title" => fields.title = scalar(value)?,
            "summary" => fields.summary = Some(scalar(value)?),
            "category" => fields.category = Some(scalar(value)?),
            "cover" => fields.cover = Some(scalar(value)?),
            "tags" if list => {}
            "tags" => fields.tags = tags(value)?,
            _ => return Err(invalid(format!("Unsupported frontmatter field: {key}"))),
        }
    }
    Err(invalid("Article frontmatter needs a closing --- line"))
}

fn scalar(value: &str) -> Result<String> {
    let value = value.trim();
    if value.starts_with('"') {
        return serde_json::from_str(value).map_err(|_| invalid("Invalid quoted value"));
    }
    if let Some(quoted) = value.strip_prefix('\'') {
        let quoted = quoted
            .strip_suffix('\'')
            .ok_or_else(|| invalid("Invalid quoted frontmatter value"))?;
        return Ok(quoted.replace("''", "'"));
    }
    Ok(value.into())
}

fn push_tag(tags: &mut Vec<String>, value: &str) -> Result<()> {
    let tag = scalar(value)?;
    if tag.trim().is_empty() {
        return Err(invalid("Article tags cannot be empty"));
    }
    tags.push(tag);
    Ok(())
}

fn tags(value: &str) -> Result<Vec<String>> {
    let mut tags = Vec::new();
    let Some(list) = value.strip_prefix('[') else {
        push_tag(&mut tags, value)?;
        return Ok(tags);
    };
    let list = list
        .strip_suffix(']')
        .ok_or_else(|| invalid("Inline tags require a closing ]"))?;
    let mut quote = None;
    let mut escaped = false;
    let mut start = 0;
    let mut characters = list.char_indices().peekable();
    while let Some((index, character)) = characters.next() {
        if escaped {
            escaped = false;
        } else if quote == Some('"') && character == '\\' {
            escaped = true;
        } else if quote == Some(character) {
            if character == '\'' && characters.next_if(|&(_, next)| next == '\'').is_some() {
                continue;
            }
            quote = None;
        } else if quote.is_none() {
            match character {
                '\'' | '"' if list[start..index].trim().is_empty() => quote = Some(character),
                ',' => {
                    push_tag(&mut tags, &list[start..index])?;
                    start = index + 1;
                }
                _ => {}
            }
        }
    }
    if quote.is_some() {
        return Err(invalid("Unclosed quoted tag"));
    }
    if !list[start..].trim().is_empty() {
        push_tag(&mut tags, &list[start..])?;
    }
    Ok(tags)
}

fn local_path(destination: &str) -> Result<String> {
    let rejected = || {
        invalid(format!(
            "Expected a relative local image path: {destination}"
        ))
    };
    if destination.contains(['?', '#']) {
        return Err(rejected());
    }
    let mut bytes = Vec::new();
    let mut input = destination.bytes();
    while let Some(byte) = input.next() {
        if byte == b'%' {
            let high = input.next().and_then(|b| char::from(b).to_digit(16));
            let low = input.next().and_then(|b| char::from(b).to_digit(16));
            let (Some(high), Some(low)) = (high, low) else {
                return Err(rejected());
            };
            bytes.push((high * 16 + low) as u8);
        } else {
            bytes.push(byte);
        }
    }
    let decoded = String::from_utf8(bytes).map_err(|_| rejected())?;
    if decoded.is_empty()
        || decoded.contains(['\\', ':'])
        || decoded.chars().any(char::is_control)
        || Path::new(&decoded)
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(rejected());
    }
    Ok(decoded)
}

fn image_type(path: &Path) -> Result<(String, &'static str)> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let media_type = match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "tif" | "tiff" => "image/tiff",
        _ => {
            return Err(invalid(format!(
                "Unsupported image extension: {}",
                path.display()
            )));
        }
    };
    Ok((extension, media_type))
}

fn safe_stem(path: &Path) -> String {
    let stem: String = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .chars()
        .take(80)
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let stem = stem.trim_matches('-');
    if stem.is_empty() {
        "image".into()
    } else {
        stem.into()
    }
}

fn rewrite_images(
    body: &str,
    asset_path: &mut impl FnMut(&str) -> Result<String>,
) -> Result<String> {
    let mut rewritten = String::with_capacity(body.len());
    let mut fence = None;
    for line in body.split_inclusive('\n') {
        let unindented = line.trim_start_matches(' ');
        let marker = unindented.as_bytes().first().copied();
        if line.len() - unindented.len() <= 3
            && let Some(marker @ (b'`' | b'~')) = marker
        {
            let size = marker_run(unindented.as_bytes(), marker);
            if size >= 3 {
                if fence.is_none() {
                    fence = Some((marker, size));
                    rewritten.push_str(line);
                    continue;
                }
                if let Some((active, minimum)) = fence
                    && marker == active
                    && size >= minimum
                    && unindented[size..].trim().is_empty()
                {
                    fence = None;
                    rewritten.push_str(line);
                    continue;
                }
            }
        }
        if fence.is_some() || line.starts_with("    ") || line.starts_with('\t') {
            rewritten.push_str(line);
        } else {
            rewrite_line(line, &mut rewritten, asset_path)?;
        }
    }
    Ok(rewritten)
}

fn marker_run(bytes: &[u8], marker: u8) -> usize {
    bytes.iter().take_while(|&&byte| byte == marker).count()
}

fn rewrite_line(
    line: &str,
    rewritten: &mut String,
    asset_path: &mut impl FnMut(&str) -> Result<String>,
) -> Result<()> {
    let bytes = line.as_bytes();
    let mut index = 0;
    let mut copied = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if bytes.get(index + 1).is_some_and(u8::is_ascii_punctuation) => index += 2,
            b'`' => {
                let size = marker_run(&bytes[index..], b'`');
                index += size;
                let mut rest = index;
                while let Some(offset) = line[rest..].find('`') {
                    let start = rest + offset;
                    let closing = marker_run(&bytes[start..], b'`');
                    rest = start + closing;
                    if closing == size {
                        index = rest;
                        break;
                    }
                }
            }
            b'!' if bytes.get(index + 1) == Some(&b'[') => {
                if let Some(open) = image_open(&bytes[index + 2..]) {
                    let start = index + 2 + open;
                    let (from, to, next) = image_destination(&line[start..])?;
                    rewritten.push_str(&line[copied..start + from]);
                    rewritten.push_str(&asset_path(&line[start + from..start + to])?);
                    copied = start + to;
                    index = start + next;
                } else {
                    index += 2;
                }
            }
            _ => index += 1,
        }
    }
    rewritten.push_str(&line[copied..]);
    Ok(())
}

fn image_open(alt: &[u8]) -> Option<usize> {
    let mut depth = 0;
    let mut escaped = false;
    for (index, &byte) in alt.iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match byte {
            b'\\' => escaped = true,
            b'[' => depth += 1,
            b']' if depth == 0 => {
                return (alt.get(index + 1) == Some(&b'(')).then_some(index + 2);
            }
            b']' => depth -= 1,
            _ => {}
        }
    }
    None
}

fn image_destination(input: &str) -> Result<(usize, usize, usize)> {
    let malformed = || invalid("Malformed inline Markdown image destination");
    let trimmed = input.trim_start_matches([' ', '\t']);
    let start = input.len() - trimmed.len();
    let (from, to, end) = if let Some(angled) = trimmed.strip_prefix('<') {
        let close = angled.find('>').ok_or_else(malformed)?;
        (start + 1, start + 1 + close, start + 2 + close)
    } else {
        let mut depth = 0;
        let path_end = trimmed
            .bytes()
            .position(|byte| match byte {
                b'(' => {
                    depth += 1;
                    false
                }
                b')' if depth > 0 => {
                    depth -= 1;
                    false
                }
                b')' | b' ' | b'\t' | b'\n' => true,
                _ => false,
            })
            .filter(|_| depth == 0)
            .ok_or_else(malformed)?;
        (start, start + path_end, start + path_end)
    };
    let rest = input[end..].trim_start_matches([' ', '\t']);
    let closing = if rest.starts_with(')') {
        rest
    } else {
        let (first, title) = rest.split_at_checked(1).ok_or_else(malformed)?;
        let delimiter = match first {
            "\"" => '"',
            "'" => '\'',
            "(" => ')',
            _ => return Err(malformed()),
        };
        let mut escaped = false;
        let close = title
            .find(|character| {
                if escaped {
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == delimiter {
                    return true;
                }
                false
            })
            .ok_or_else(malformed)?;
        title[close + 1..].trim_start_matches([' ', '\t'])
    };
    if !closing.starts_with(')') {
        return Err(malformed());
    }
    Ok((from, to, input.len() - closing.len() + 1))
}

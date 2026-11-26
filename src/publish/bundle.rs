use super::{Article, Asset, PublisherAdapter, Result, invalid, sha256};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Serialize)]
struct Manifest<'a> {
    schema_version: u32,
    article_id: &'a str,
    platform: &'a str,
    title: &'a str,
    summary: Option<&'a str>,
    tags: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    topic_suggestions: Option<&'a [String]>,
    category: Option<&'a str>,
    cover: Option<&'a str>,
    source_sha256: &'a str,
    normalized_body_sha256: String,
    assets: &'a [Asset],
}

pub struct Bundle<'a> {
    article_id: &'a str,
    platform: &'a str,
    files: BTreeMap<&'a str, Vec<u8>>,
    pub(super) canonical: Vec<u8>,
    digest: String,
}

impl<'a> Bundle<'a> {
    pub fn build(article: &'a Article, adapter: &'a dyn PublisherAdapter) -> Result<Self> {
        let platform = adapter.id();
        if platform.is_empty()
            || !platform
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(invalid("Invalid platform ID"));
        }
        let normalized = adapter.normalize(article)?;
        let topic_suggestions = adapter
            .tags_are_topic_suggestions()
            .then_some(normalized.tags.as_slice());
        let record = article.metadata();
        let manifest = Manifest {
            schema_version: 1,
            article_id: &record.id,
            platform,
            title: &normalized.title,
            summary: normalized.summary.as_deref(),
            tags: if topic_suggestions.is_some() {
                &[]
            } else {
                &normalized.tags
            },
            topic_suggestions,
            category: normalized.category.as_deref(),
            cover: record.cover.as_deref(),
            source_sha256: &record.source_sha256,
            normalized_body_sha256: sha256(normalized.markdown.as_bytes()),
            assets: &record.assets,
        };
        let checklist = checklist(&manifest, adapter);
        let mut manifest_json = serde_json::to_vec_pretty(&manifest)?;
        manifest_json.push(b'\n');
        let mut files = BTreeMap::from([
            ("manifest.json", manifest_json),
            ("article.md", normalized.markdown.into_bytes()),
            ("checklist.md", checklist.into_bytes()),
        ]);
        for (asset, bytes) in record.assets.iter().zip(&article.asset_bytes) {
            files.insert(asset.path.as_str(), bytes.clone());
        }
        let mut canonical = Vec::new();
        for (path, bytes) in &files {
            // Sort by path and prefix each value with its length to fix file boundaries.
            canonical.extend_from_slice(&(path.len() as u64).to_be_bytes());
            canonical.extend_from_slice(path.as_bytes());
            canonical.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
            canonical.extend_from_slice(bytes);
        }
        let digest = sha256(&canonical);
        Ok(Self {
            article_id: &record.id,
            platform,
            files,
            canonical,
            digest,
        })
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn write(&self, out: &Path) -> Result<PathBuf> {
        self.stage(out)?.finish()
    }

    pub(super) fn stage(&self, out: &Path) -> Result<BundleExport<'_>> {
        let parent = out.join(self.article_id);
        let destination = parent.join(self.platform);
        let mut export = BundleExport {
            bundle: self,
            destination,
            staging: None,
        };
        if export.existing_matches()? {
            return Ok(export);
        }
        fs::create_dir_all(&parent)?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let staging = parent.join(format!(".package-{}-{nonce}", std::process::id()));
        fs::create_dir(&staging)?;
        export.staging = Some(staging.clone());
        fs::create_dir(staging.join("assets"))?;
        for (path, content) in &self.files {
            fs::write(staging.join(path), content)?;
        }
        Ok(export)
    }
}

pub(super) struct BundleExport<'a> {
    bundle: &'a Bundle<'a>,
    destination: PathBuf,
    staging: Option<PathBuf>,
}

impl BundleExport<'_> {
    fn existing_matches(&self) -> Result<bool> {
        let metadata = match self.destination.symlink_metadata() {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        let conflict = || {
            invalid(format!(
                "Bundle destination already contains different files: {}",
                self.destination.display()
            ))
        };
        let assets = self.destination.join("assets");
        if !metadata.is_dir()
            || fs::read_dir(&self.destination)?.count() != 4
            || !assets.symlink_metadata()?.is_dir()
            || fs::read_dir(assets)?.count() != self.bundle.files.len() - 3
        {
            return Err(conflict());
        }
        for (path, content) in &self.bundle.files {
            let existing = self.destination.join(path);
            if !existing.symlink_metadata()?.is_file() || &fs::read(existing)? != content {
                return Err(conflict());
            }
        }
        Ok(true)
    }

    pub(super) fn finish(mut self) -> Result<PathBuf> {
        if let Some(staging) = &self.staging {
            if !self.existing_matches()?
                && let Err(error) = fs::rename(staging, &self.destination)
                && !self.existing_matches()?
            {
                return Err(error.into());
            }
        } else if !self.existing_matches()? {
            return Err(invalid(
                "Existing bundle disappeared while preparing the export",
            ));
        }
        Ok(std::mem::take(&mut self.destination))
    }
}

impl Drop for BundleExport<'_> {
    fn drop(&mut self) {
        if let Some(staging) = &self.staging {
            let _ = fs::remove_dir_all(staging);
        }
    }
}

fn checklist(manifest: &Manifest<'_>, adapter: &dyn PublisherAdapter) -> String {
    let creator = adapter
        .creator_url()
        .unwrap_or("Open your chosen platform’s creator editor");
    let title = manifest.title;
    let summary = manifest.summary.unwrap_or("None");
    let (label, tags) = match manifest.topic_suggestions {
        Some(topics) => ("Topic suggestions", topics),
        None => ("Tags", manifest.tags),
    };
    let tags = if tags.is_empty() {
        "None".into()
    } else {
        tags.join(", ")
    };
    let cover = manifest.cover.unwrap_or("None");
    let mut markdown = format!(
        "# Publication checklist\n\nCreator editor: {creator}\n\n\
         - Title: {title}\n- Summary: {summary}\n- {label}: {tags}\n"
    );
    if manifest.topic_suggestions.is_none() {
        let category = manifest.category.unwrap_or("None");
        markdown.push_str(&format!("- Category: {category}\n"));
    }
    markdown.push_str(&format!(
        "- Cover: {cover}\n\n\
         Paste `article.md` into the editor and upload the images in this order:\n"
    ));
    for asset in manifest.assets {
        markdown.push_str("- [ ] `");
        markdown.push_str(&asset.path);
        markdown.push_str("`\n");
    }
    if manifest.assets.is_empty() {
        markdown.push_str("\nNo images to upload.\n");
    }
    let steps = adapter.checklist_steps();
    if !steps.is_empty() {
        markdown.push('\n');
        for step in steps {
            markdown.push_str("- [ ] ");
            markdown.push_str(step);
            markdown.push('\n');
        }
        markdown.push_str("- [ ] Paste the live public URL into `kirzner publish record`.\n");
    } else {
        markdown.push_str("\n- [ ] Paste the live public URL: \n");
    }
    markdown
}

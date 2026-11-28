// Portions ported from ArtiPub, Copyright (c) 2019 Crawlab Team.
// Licensed under the BSD-3-Clause License; see THIRD_PARTY_NOTICES.md.
mod approval;
mod bluesky;
mod card;
mod format;
mod github;
mod run;
mod workspace;

pub use approval::{CampaignAction, approval_batch, approve_campaign, approved, in_window};
pub use bluesky::Bluesky;
pub use format::{
    ActionRecord, Approval, Asset, BlueskyIntent, Campaign, ExecutionEntry, Intent, Milestone,
    Plan, PlanItem, RecordStatus, ReleaseIntent, RetiredIntent, Step, append_execution,
    parse_campaign,
};
pub use github::GitHubReleases;
pub use run::{CampaignResult, reconcile_absent, run_campaign};
pub use workspace::{
    CampaignStore, campaign_approve, campaign_plan, campaign_status, read_campaign, with_campaign,
};

fn provider_client(builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    builder
        .no_proxy()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .timeout(std::time::Duration::from_secs(15))
}

#[derive(Debug)]
enum BodyError {
    Request(reqwest::Error),
    TooLarge,
}

async fn bounded_body(mut response: reqwest::Response, limit: usize) -> Result<Vec<u8>, BodyError> {
    if response
        .content_length()
        .is_some_and(|size| size > limit as u64)
    {
        return Err(BodyError::TooLarge);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(BodyError::Request)? {
        if chunk.len() > limit - body.len() {
            return Err(BodyError::TooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("current time fits RFC 3339")
}

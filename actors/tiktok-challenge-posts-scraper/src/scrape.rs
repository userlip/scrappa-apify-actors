use crate::{
    input::ChallengeRequest,
    ports::{PushResult, ResultsSink},
    response::{get_video_id, js_string, parse_page},
    scrappa::{PostsParams, ScrappaApi},
};
use chrono::{SecondsFormat, Utc};
use serde_json::{json, Value};
use std::collections::HashSet;

pub const MAX_PAGES_PER_CHALLENGE: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeSummary {
    pub challenge_id: String,
    pub status: &'static str,
    pub videos_saved: usize,
    pub pages_fetched: usize,
    pub next_cursor: Option<String>,
    pub error: Option<String>,
}

impl ChallengeSummary {
    pub fn to_json(&self) -> Value {
        let mut value = json!({
            "challenge_id": self.challenge_id,
            "status": self.status,
            "videos_saved": self.videos_saved,
            "pages_fetched": self.pages_fetched,
            "next_cursor": self.next_cursor,
        });
        if let Some(error) = &self.error {
            value["error"] = Value::String(error.clone());
        }
        value
    }
}

pub async fn scrape_challenge<C, S>(
    client: &C,
    sink: &mut S,
    request: &ChallengeRequest,
    seen_ids: &mut HashSet<String>,
) -> ChallengeSummary
where
    C: ScrappaApi,
    S: ResultsSink,
{
    let mut seen_cursors = HashSet::new();
    if let Some(cursor) = &request.initial_cursor {
        seen_cursors.insert(cursor.clone());
    }
    let mut cursor = request.initial_cursor.clone();
    let mut saved = 0;
    let mut pages = 0;

    while saved < request.result_limit && pages < MAX_PAGES_PER_CHALLENGE {
        let capacity = sink.available_capacity(request.result_limit - saved);
        if capacity == 0 {
            return summary(request, "charge-limit-reached", saved, pages, cursor, None);
        }

        let count = request
            .page_size
            .min(capacity)
            .min(request.result_limit - saved);
        let response = match client
            .get_posts(&PostsParams {
                challenge_id: request.challenge_id.clone(),
                count,
                region: request.region.clone(),
                cursor: cursor.clone(),
            })
            .await
        {
            Ok(response) => response,
            Err(error) => {
                return summary(
                    request,
                    "failed",
                    saved,
                    pages,
                    cursor,
                    Some(error.to_string()),
                );
            }
        };
        pages += 1;

        if let Some(code) = response.get("code") {
            if code.as_f64() != Some(0.0) {
                let message = response
                    .get("msg")
                    .filter(|message| !message.is_null())
                    .map(js_string)
                    .unwrap_or_else(|| "Unknown error".to_owned());
                let error = format!("Scrappa API code {}: {message}", js_string(code));
                return summary(request, "failed", saved, pages, cursor, Some(error));
            }
        }

        let page = parse_page(response.get("data"));
        let mut page_ids = HashSet::new();
        let unique_videos = page
            .videos
            .into_iter()
            .filter(|video| {
                let Some(id) = get_video_id(video) else {
                    return false;
                };
                !seen_ids.contains(&id) && page_ids.insert(id)
            })
            .take(count);
        let rows = unique_videos
            .map(|mut video| {
                video.insert(
                    "challenge_id".to_owned(),
                    Value::String(request.challenge_id.clone()),
                );
                video.insert(
                    "requested_region".to_owned(),
                    request
                        .region
                        .clone()
                        .map(Value::String)
                        .unwrap_or(Value::Null),
                );
                video.insert(
                    "scraped_at".to_owned(),
                    Value::String(Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)),
                );
                Value::Object(video)
            })
            .collect::<Vec<_>>();

        let push = match sink.push_videos(&rows).await {
            Ok(push) => push,
            Err(error) => {
                return summary(
                    request,
                    "failed",
                    saved,
                    pages,
                    cursor,
                    Some(error.to_string()),
                );
            }
        };
        remember_saved_ids(&rows, push, seen_ids);
        saved += push.saved;
        cursor = page.cursor;

        if push.limit_reached {
            return summary(request, "charge-limit-reached", saved, pages, cursor, None);
        }
        if !page.has_more || cursor.is_none() {
            return summary(request, "succeeded", saved, pages, cursor, None);
        }
        let next_cursor = cursor.as_ref().expect("cursor checked above");
        if !seen_cursors.insert(next_cursor.clone()) {
            return summary(request, "pagination-stalled", saved, pages, cursor, None);
        }
    }

    let status = if saved >= request.result_limit {
        "succeeded"
    } else {
        "page-limit-reached"
    };
    summary(request, status, saved, pages, cursor, None)
}

fn remember_saved_ids(rows: &[Value], result: PushResult, seen_ids: &mut HashSet<String>) {
    for row in rows.iter().take(result.saved) {
        if let Some(id) = row.as_object().and_then(get_video_id) {
            seen_ids.insert(id);
        }
    }
}

fn summary(
    request: &ChallengeRequest,
    status: &'static str,
    videos_saved: usize,
    pages_fetched: usize,
    next_cursor: Option<String>,
    error: Option<String>,
) -> ChallengeSummary {
    ChallengeSummary {
        challenge_id: request.challenge_id.clone(),
        status,
        videos_saved,
        pages_fetched,
        next_cursor,
        error,
    }
}

pub fn is_total_failure(summaries: &[ChallengeSummary]) -> bool {
    !summaries.is_empty()
        && summaries
            .iter()
            .all(|summary| summary.status == "failed" && summary.videos_saved == 0)
}

# TikTok Hashtag Videos Scraper

Collect public TikTok videos for multiple numeric challenge IDs in one Apify run. The Actor is a lightweight wrapper around Scrappa's `tiktok-challenges-posts` API and returns one dataset item per unique video for campaign monitoring, hashtag research, creator discovery, and engagement analysis.

## TikTok challenge workflow

1. **Search:** use the TikTok Challenge Search Actor to find hashtag challenge IDs.
2. **Details:** use the TikTok Challenge Details Actor when you need challenge metadata.
3. **Posts:** pass those IDs to this Actor to collect videos with captions, author details, covers, media URLs, music, duration, region, timestamps, and engagement counts.

For larger workloads, lower latency, or direct integration, use the [Scrappa API](https://scrappa.co) and call `tiktok-challenges-posts` directly.

## Input

```json
{
  "challenge_ids": ["1622962893630470", "7559525500510173270"],
  "region": "US",
  "results_per_challenge": 100,
  "page_size": 10
}
```

`challenge_id` remains available for single-ID compatibility. A run accepts at most 20 IDs, 500 results per challenge, and 2,000 requested results overall. Pagination is bounded by these limits and the Apify event-charge limit.

## Output and pricing

Every unique successful video is one dataset row and one `challenge-post-result` paid event at **$0.00025 per video** ($0.25 per 1,000 results). Failed challenge IDs are isolated so other IDs can still finish. Dataset rows include `challenge_id`, `requested_region`, and `scraped_at` provenance.

TikTok cover, video, avatar, and music URLs may be signed and expire. Store media you are authorized to retain promptly. Stable `video_id`/`aweme_id` values are suitable for deduplication and monitoring.

No per-video key-value-store records are written; the default dataset is the result channel.

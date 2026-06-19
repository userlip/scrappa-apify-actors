# 2026-06-19 TikTok Paid No-Run Actor Smoke Sweep

Scope: smoke-run the four public paid TikTok actors that had no successful run history: `tiktok-ads-scraper`, `tiktok-comments-scraper`, `tiktok-hashtag-posts-scraper`, and `tiktok-search-scraper`. Inputs were minimal documented examples and all scraping stayed on Scrappa API infrastructure. No actor source, batching behavior, pricing, deployment, or listing metadata was changed.

Pre-run validation: live Apify API metadata showed all four target actors were public, had `SCRAPPA_API_KEY` present as a secret on version `1.0`, had `PAY_PER_EVENT` pricing entries for the `apify-default-dataset-item` event, had successful latest builds, and had no previous runs.

Post-run validation: `APIFY_TOKEN=... node scripts/audit-apify-health.mjs --json` reported the owner-filtered portfolio summary moved to `32` OK and `47` no-runs. The same audit reported all four target actors as `OK`, each with latest run status `SUCCEEDED`. The portfolio no-run count dropped from the task-provided `51` to `47`, matching these four first successful runs.

| Actor | Actor ID | Pricing status | Secret check | Latest build | Smoke input | Run ID | Status | Default dataset items | Charged event count | Log/result evidence |
| --- | --- | --- | --- | --- | --- | --- | --- | ---: | ---: | --- |
| `tiktok-ads-scraper` | `c1Lt3EdNOuoen5In7` | Paid `PAY_PER_EVENT`, `apify-default-dataset-item` at `$0.0002` | `SCRAPPA_API_KEY` present as secret on version `1.0` | `2vQDoQ9UnhdTdXBqJ` (`SUCCEEDED`) | `urls: [https://ads.tiktok.com/business/creativecenter/topads/7213160569871581185/pc/en?countryCode=US&period=30]` | `cRp2U8IFSKPAcdeA6` | `SUCCEEDED` | 1 | 1 | Log reported `Found 1 TikTok ad record` and `dataset_items: 1`. Dataset item included `ad_id: 7213160569871581185`, `request_url`, `request_ad_id`, `result_found: true`, and engagement fields. |
| `tiktok-comments-scraper` | `oaJANlheGg9o3EZjU` | Paid `PAY_PER_EVENT`, primary `apify-default-dataset-item` event at `$0.0003` | `SCRAPPA_API_KEY` present as secret on version `1.0` | `iqfDkvScBHaeu6qwV` (`SUCCEEDED`) | `url: https://www.tiktok.com/@tiktok/video/7568510388342443294`, `count: 1`, `includeReplies: false` | `qRxgfvDhXgwCpsBnY` | `SUCCEEDED` | 1 | 1 | Log reported `Found 1 comments and 0 replies` and `dataset_items: 1`. Dataset item included `comment_type: comment`, `video_id: 7568510388342443294`, `comment_id`, `text`, and user metadata. |
| `tiktok-hashtag-posts-scraper` | `H2dZTreGZ7s3XJsQ7` | Paid `PAY_PER_EVENT`, `apify-default-dataset-item` at `$0.0003` | `SCRAPPA_API_KEY` present as secret on version `1.0` | `adY4fybFQ8GgZ2iXQ` (`SUCCEEDED`) | `hashtag: cosplay`, `region: US`, `count: 1`, `cursor: 0` | `3nNxnjnbZuPSgpMZr` | `SUCCEEDED` | 1 | 1 | Log reported `Resolved hashtag to challenge_id:33380 (Cosplay)`, `Found 1 posts`, and `posts_extracted: 1`. Dataset item included `video_id`, `title`, engagement counts, `lookup_challenge_name: cosplay`, and `resolved_challenge_id: 33380`. |
| `tiktok-search-scraper` | `0h6AKrgNYjn7pM5EO` | Paid `PAY_PER_EVENT`, `apify-default-dataset-item` at `$0.0002` | `SCRAPPA_API_KEY` present as secret on version `1.0` | `QEWgP1JmVRc3kgvhb` (`SUCCEEDED`) | `keywords: basketball`, `region: US`, `count: 1`, `cursor: 0` | `uRsoeYTEEe0ovsbql` | `SUCCEEDED` | 1 | 1 | Log reported `Found 1 TikTok search results` and `videos_extracted: 1`. Dataset item included `video_id`, `title`, engagement counts, `request_keywords: basketball`, `request_region: US`, and `request_count: 1`. |

Run detail checks:

- All four run logs were clean: no uncaught exception, no Scrappa API key/config error, and no unresolved Scrappa API error.
- Each run record included active run-time `pricingInfo` with `pricingModel: PAY_PER_EVENT`.
- Each run record included `chargedEventCounts.apify-default-dataset-item: 1`, matching the one dataset item written by each smoke run.
- Runtime and memory stayed small for three wrappers: ads ran in about `4.8s`, comments in about `9.2s`, and search in about `11.0s` with `128 MB` default memory. Hashtag posts succeeded in about `5.1s`, but its live default run options still show `4096 MB` memory and `3600s` timeout even though the wrapper only made Scrappa API calls.

Audit rows after smoke:

| Actor | Audit status | Latest run | Latest build | Recent statuses |
| --- | --- | --- | --- | --- |
| `tiktok-ads-scraper` | `OK` | `cRp2U8IFSKPAcdeA6` (`SUCCEEDED`) | `2vQDoQ9UnhdTdXBqJ` (`SUCCEEDED`, build `1.0.5`) | `SUCCEEDED` |
| `tiktok-comments-scraper` | `OK` | `qRxgfvDhXgwCpsBnY` (`SUCCEEDED`) | `iqfDkvScBHaeu6qwV` (`SUCCEEDED`, build `1.0.5`) | `SUCCEEDED` |
| `tiktok-hashtag-posts-scraper` | `OK` | `3nNxnjnbZuPSgpMZr` (`SUCCEEDED`) | `adY4fybFQ8GgZ2iXQ` (`SUCCEEDED`, build `1.0.6`) | `SUCCEEDED` |
| `tiktok-search-scraper` | `OK` | `uRsoeYTEEe0ovsbql` (`SUCCEEDED`) | `QEWgP1JmVRc3kgvhb` (`SUCCEEDED`, build `1.0.4`) | `SUCCEEDED` |

Notes:

- No source-code fixes were required, so no actor deployment was performed and no code-fix pull request was created.
- `tiktok-music-posts-scraper` was intentionally excluded because it already had a successful latest run and was outside this first-run gate.
- Follow-up opportunity: reduce `tiktok-hashtag-posts-scraper` live default run options from `4096 MB` / `3600s` to a wrapper-sized setting such as `128 MB` / `300s`, matching the Apify cost-control guidance. This was not changed during this smoke run because the task scope was first-run verification and narrow wrapper bug fixes only.

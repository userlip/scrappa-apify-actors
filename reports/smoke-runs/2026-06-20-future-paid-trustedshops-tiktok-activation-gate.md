# 2026-06-20 Future-Paid TrustedShops And TikTok Activation Gate

Checked at `2026-06-20T21:16:41Z`. Both scheduled Apify pricing activation windows are still in the future, so post-activation smoke runs and paid charge-event verification are intentionally blocked today.

## Executive Summary

- Public actor `trustedshops-reviews-scraper` (`L5tTNPlxeCTlFUUjl`) is still `FUTURE_ONLY_PAID_PRICING`; paid `PAY_PER_EVENT` event `review-result` at `$0.00025` starts `2026-07-01T09:48:10.000Z`.
- Public actor `tiktok-challenge-search-scraper` (`MM5bzu7V8yORRFTqW`) is still `FUTURE_ONLY_PAID_PRICING`; paid `PAY_PER_EVENT` event `challenge-result` at `$0.00025` starts `2026-07-03T13:06:46.000Z`.
- The portfolio pricing audit currently reports `80` active paid public actors, `2` future-only paid public actors, `0` missing paid pricing, `0` overdue missing active pricing, and `0` errors.
- Recent runs for both target actors are successful and both version `1.0` configurations include `SCRAPPA_API_KEY` as an Apify secret.
- Concrete follow-ups are active: Mimir schedule `29` verifies TrustedShops at `2026-07-01 09:55 UTC`; Mimir schedule `31` verifies TikTok at `2026-07-03 13:15 UTC`.

## Detailed Report

### Data Sources Used

| Source | Evidence |
| --- | --- |
| Local clock | `date -u` returned `2026-06-20T21:16:41Z` |
| Pricing audit | `APIFY_TOKEN=... node scripts/audit-apify-pricing.mjs --json --include-active` |
| Actor details | `GET /v2/acts/L5tTNPlxeCTlFUUjl`, `GET /v2/acts/MM5bzu7V8yORRFTqW` |
| Recent runs | `GET /v2/acts/{actorId}/runs?limit=5&desc=true` |
| Actor version env vars | `GET /v2/acts/{actorId}/versions/1.0` |
| Input fixtures | `actors/trustedshops-reviews-scraper/.actor/input_schema.json`, `actors/tiktok-challenge-search-scraper/.actor/input_schema.json` |
| Scheduling | Mimir control-plane `schedules.list`, `schedules.create` |

### Monetization And Health

| Actor ID | Slug | Pricing status | Active paid evidence today | Scheduled paid pricing | Recent run health | Env var status |
| --- | --- | --- | --- | --- | --- | --- |
| `L5tTNPlxeCTlFUUjl` | `trustedshops-reviews-scraper` | `FUTURE_ONLY_PAID_PRICING` | `pricingInfo` missing; `currentPricingInfo` missing | `PAY_PER_EVENT` `review-result`, `$0.00025`, starts `2026-07-01T09:48:10.000Z` | Latest run `2QAVwmWaGinRF3nDu` succeeded on build `1.0.5`; dataset `BLqvjA2UgxyVEyGdr` | `SCRAPPA_API_KEY` present as secret on version `1.0` |
| `MM5bzu7V8yORRFTqW` | `tiktok-challenge-search-scraper` | `FUTURE_ONLY_PAID_PRICING` | `pricingInfo` missing; `currentPricingInfo` missing | `PAY_PER_EVENT` `challenge-result`, `$0.00025`, starts `2026-07-03T13:06:46.000Z` | Latest run `7bH0WvU0S6bCoNKN5` succeeded on build `1.0.3`; dataset `4TyfSYeSjA2E6IicU` | `SCRAPPA_API_KEY` present as secret on version `1.0` |

### Smoke Run Decision

No post-activation smoke runs were started on `2026-06-20`, because Apify cannot provide valid active paid pricing or `chargedEventCounts` evidence before the configured `startedAt` timestamps.

Required post-activation smoke inputs:

```json
{
  "tsids": ["XFB15FFBDE1DEE7A55D292A7D48598A6A"],
  "page": 1,
  "max_pages": 1,
  "size": 1
}
```

```json
{
  "keywords": ["fitness"],
  "count": 1
}
```

### Scheduled Follow-Up

| Schedule ID | Name | Cron | Purpose |
| --- | --- | --- | --- |
| `29` | `Verify TrustedShops Reviews paid activation` | `55 9 1 7 *` | Verify `L5tTNPlxeCTlFUUjl` after `2026-07-01T09:48:10.000Z`, then smoke-run and record `review-result` charge behavior |
| `31` | `Verify TikTok Challenge Search paid activation` | `15 13 3 7 *` | Verify `MM5bzu7V8yORRFTqW` after `2026-07-03T13:06:46.000Z`, then smoke-run and record `challenge-result` charge behavior |

### Risk And Verification

The only current blocker is time: both actors are public and scheduled for paid pricing, but neither scheduled `startedAt` has occurred. After each timestamp, verification must confirm that the actor leaves `FUTURE_ONLY_PAID_PRICING`, has started paid evidence in Apify pricing fields, succeeds on a minimal smoke run, writes usable dataset output or a documented valid zero-result response, and reports the expected charge event when dataset items are saved.

No actor code, listing copy, pricing configuration, Apify secret, or deployment was changed during this check.

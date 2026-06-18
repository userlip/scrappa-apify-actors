# 2026-07-01 TrustedShops Reviews Paid Activation Gate

Status: scheduled. This report is intentionally pending until after the Apify pricing start time `2026-07-01T09:48:10.000Z`.

Mimir schedule: `29` (`Verify TrustedShops Reviews paid activation`) runs at `55 9 1 7 *` for repository `18` with the Scrappa Apify Growth Strategist persona. The first intended run is shortly after `2026-07-01T09:48:10.000Z`.

Scope: verify that public actor `trustedshops-reviews-scraper` (`L5tTNPlxeCTlFUUjl`) moves from future-only paid pricing to active paid pricing, then smoke-run one small TrustedShops review request and confirm `review-result` charge-event evidence.

## Pre-Activation Evidence

Checked at `2026-06-18T08:00:18.497Z` with `APIFY_TOKEN=... node scripts/audit-apify-pricing.mjs --json`.

Portfolio summary:

| Status | Count |
| --- | ---: |
| `ACTIVE_PAID_PRICING` | 79 |
| `OVERDUE_MISSING_ACTIVE_PRICING` | 0 |
| `MISSING_PAID_PRICING` | 0 |
| `FUTURE_ONLY_PAID_PRICING` | 2 |
| `ERROR` | 0 |

Actor pricing evidence:

| Actor | Actor ID | Public | Current status | `pricingInfo` | `currentPricingInfo` | Scheduled paid pricing |
| --- | --- | --- | --- | --- | --- | --- |
| `trustedshops-reviews-scraper` | `L5tTNPlxeCTlFUUjl` | Yes | `FUTURE_ONLY_PAID_PRICING` | Missing | Missing | `PAY_PER_EVENT` starts `2026-07-01T09:48:10.000Z`; event `review-result`; event title `Review result`; event price `$0.00025`; primary event true |

Live actor detail fetched on `2026-06-18` also reported:

| Field | Evidence |
| --- | --- |
| Actor title | `TrustedShops Reviews Scraper` |
| Modified at | `2026-06-17T15:50:04.469Z` |
| Latest build | `yl37FrXUI3IIQHEVT`, build `1.0.5`, `SUCCEEDED`, finished `2026-06-17T15:50:04.469Z` |
| Latest run | `2QAVwmWaGinRF3nDu`, `SUCCEEDED`, dataset `BLqvjA2UgxyVEyGdr`, finished `2026-06-17T15:50:38.096Z` |
| Version secret | `SCRAPPA_API_KEY` present as secret on version `1.0` |
| Charge evidence before activation | Not available. Recent pre-activation runs reported `chargedEventCounts: null`, which is expected while active pricing is not yet available. |

## Post-Activation Verification Steps

Run these after `2026-07-01T09:48:10.000Z`.

1. Re-run the portfolio pricing audit:

```bash
APIFY_TOKEN=... node scripts/audit-apify-pricing.mjs --json > /tmp/apify-pricing-audit-2026-07-01.json
```

2. Confirm `trustedshops-reviews-scraper` is no longer `FUTURE_ONLY_PAID_PRICING` and has active paid evidence for `PAY_PER_EVENT` event `review-result` at `$0.00025`.

3. Fetch actor detail directly and save the active pricing evidence:

```bash
curl -s "https://api.apify.com/v2/acts/L5tTNPlxeCTlFUUjl?token=$APIFY_TOKEN"
```

4. Run the minimal smoke input from `actors/trustedshops-reviews-scraper/.actor/input_schema.json`:

```json
{
  "tsids": ["XFB15FFBDE1DEE7A55D292A7D48598A6A"],
  "page": 1,
  "max_pages": 1,
  "size": 1
}
```

If that fixture is no longer valid, use the current actor README or input schema fixture and record the replacement input.

5. Inspect:

| Required evidence | Value |
| --- | --- |
| Smoke run ID | Pending |
| Run status | Pending |
| Default dataset ID | Pending |
| Default dataset item count | Pending |
| `chargedEventCounts.review-result` or equivalent event charge evidence | Pending |
| Log summary | Pending |
| `scripts/audit-apify-pricing.mjs --json` status for actor | Pending |

## Success Criteria

- Actor `L5tTNPlxeCTlFUUjl` has active paid `PAY_PER_EVENT` pricing evidence after `2026-07-01T09:48:10.000Z`.
- Active event is `review-result` at `$0.00025` per saved TrustedShops review.
- Smoke run succeeds and saves TrustedShops review dataset items, or records a valid zero-result response.
- When dataset items are saved, run charge evidence reports `review-result` correctly.
- The pricing audit no longer lists `trustedshops-reviews-scraper` as `FUTURE_ONLY_PAID_PRICING`.

## Current Decision

No actor code, pricing configuration, listing copy, or Apify deployment was changed on `2026-06-18`. The only executable follow-up created was Mimir schedule `29`, because active paid evidence and charge-event counts cannot be verified before Apify reaches the scheduled start time.

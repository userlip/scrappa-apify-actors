# Google Trends Autocomplete Paid Activation Pre-Check

Checked at: 2026-06-18T08:00:12.036Z

## Executive Summary

- Actor `x4NPccdc4xbJZ2r6c` / `google-trends-autocomplete-scraper` is public but still future-only paid before the scheduled Apify activation time `2026-06-19T09:30:30.582Z`.
- Live Apify pricing evidence still shows no active `pricingInfo` or `currentPricingInfo`; the only paid evidence is a future `PAY_PER_EVENT` entry.
- The scheduled event is `suggestion-result`, titled `Autocomplete suggestion`, priced at `$0.0002` per saved suggestion, and marked as the primary event.
- A charge-event smoke run was not executed because any run before activation cannot prove active paid pricing or pay-per-event charging.

## Data Sources

- `GET https://api.apify.com/v2/acts/x4NPccdc4xbJZ2r6c`
- `APIFY_TOKEN=... node scripts/audit-apify-pricing.mjs --json`
- Local input schema: `actors/google-trends-autocomplete-scraper/.actor/input_schema.json`
- Local actor source: `actors/google-trends-autocomplete-scraper/src/main.ts`

## Monetization Evidence

| Actor ID | Slug | Public | Pricing status | Active evidence | Future paid evidence |
| --- | --- | --- | --- | --- | --- |
| `x4NPccdc4xbJZ2r6c` | `google-trends-autocomplete-scraper` | Yes | `FUTURE_ONLY_PAID_PRICING` | None | `PAY_PER_EVENT` starts `2026-06-19T09:30:30.582Z` |

Audit summary at `2026-06-18T08:00:12.036Z`:

| Public actors | Active paid | Missing paid | Overdue missing active | Future-only paid |
| --- | --- | --- | --- | --- |
| 81 | 79 | 0 | 0 | 2 |

Pricing fields for `x4NPccdc4xbJZ2r6c`:

| Field | Result |
| --- | --- |
| `pricingInfo` | Not present |
| `currentPricingInfo` | Not present |
| `pricingInfos[0].pricingModel` | `PAY_PER_EVENT` |
| `pricingInfos[0].startedAt` | `2026-06-19T09:30:30.582Z` |
| `pricingInfos[0].pricingPerEvent.actorChargeEvents.suggestion-result.eventPriceUsd` | `0.0002` |
| `pricingInfos[0].pricingPerEvent.actorChargeEvents.suggestion-result.isPrimaryEvent` | `true` |

## Smoke Input To Run After Activation

Use a stable autocomplete query after `2026-06-19T09:30:30.582Z`:

```json
{
  "query": "tesla",
  "geo": "US",
  "hl": "en"
}
```

The actor also accepts the direct Scrappa API alias:

```json
{
  "q": "tesla",
  "geo": "US",
  "hl": "en"
}
```

## Charge-Event Readiness

Local source `actors/google-trends-autocomplete-scraper/src/main.ts` is already wired for active Apify pay-per-event charging:

- Event constant: `suggestion-result`
- Active paid branch: `Actor.getChargingManager().getPricingInfo().isPayPerEvent`
- Dataset write with charge event: `Actor.pushData(datasetItems, SUGGESTION_RESULT_CHARGE_EVENT)`
- Saved count source: `chargeResult.chargedCount`
- Charge-limit behavior: writes `OUTPUT` with `charge_limit_reached: true` before exiting

## Required Post-Activation Verification

After `2026-06-19T09:30:30.582Z`:

1. Run `APIFY_TOKEN=... node scripts/audit-apify-pricing.mjs --json`.
2. Confirm `x4NPccdc4xbJZ2r6c` is no longer listed as `FUTURE_ONLY_PAID_PRICING`.
3. Confirm `pricingInfo`, `currentPricingInfo`, or active pricing evidence shows `PAY_PER_EVENT`.
4. Run `x4NPccdc4xbJZ2r6c` with the `tesla` smoke input above.
5. Record the run ID, final status, default dataset item count, `OUTPUT.saved_suggestion_count`, and charge-event evidence.
6. Add a post-activation report in `reports/smoke-runs/` and update `reports/smoke-runs/README.md`.

## Current Result

Blocked by time. The activation timestamp is in the future relative to this check, so the actor should remain treated as incomplete until a post-activation Apify API check and smoke run prove active paid pricing and `suggestion-result` charging.

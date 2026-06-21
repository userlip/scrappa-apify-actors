# Google Trends Autocomplete Post-Activation Smoke Run

Checked at: 2026-06-20T21:16:11.296Z

## Executive Summary

- Actor `x4NPccdc4xbJZ2r6c` / `google-trends-autocomplete-scraper` is public and has active `PAY_PER_EVENT` pricing for `suggestion-result` at `$0.0002` per autocomplete suggestion, started `2026-06-19T09:30:30.582Z`.
- Controlled smoke run `CSAByfk3493uHtVRO` succeeded on build `0asSM36C7FucorgIg` after paid activation, using input `{"query":"tesla","geo":"US","hl":"en"}`.
- The run wrote 5 default dataset rows and Apify run-level charging reports `chargedEventCounts.suggestion-result = 5`, matching the customer-visible result count.
- The post-run health audit now classifies this actor as `OK`, not `NO_RUNS`.
- Non-blocking follow-up: the actor `OUTPUT` summary reported `saved_suggestion_count = 10` while the dataset and run-level charge event count were both 5. The authoritative Apify run event count matches dataset output, so no code change was made during this smoke run.

## Data Sources

- `GET https://api.apify.com/v2/acts/x4NPccdc4xbJZ2r6c`
- `POST https://api.apify.com/v2/acts/x4NPccdc4xbJZ2r6c/runs`
- `GET https://api.apify.com/v2/actor-runs/CSAByfk3493uHtVRO`
- `GET https://api.apify.com/v2/actor-runs/CSAByfk3493uHtVRO/log`
- `GET https://api.apify.com/v2/datasets/42eqtPpK4sYgmAlVZ/items?clean=true&format=json`
- `GET https://api.apify.com/v2/key-value-stores/7mP1zebhXQx0Dpf5r/records/OUTPUT`
- `APIFY_TOKEN=... node scripts/audit-apify-pricing.mjs --json`
- `APIFY_TOKEN=... node scripts/audit-apify-health.mjs --json`
- Local schema: `actors/google-trends-autocomplete-scraper/.actor/input_schema.json`
- Local source: `actors/google-trends-autocomplete-scraper/src/main.ts`
- Local README: `actors/google-trends-autocomplete-scraper/README.md`

## Actor Monetization And Health

| Actor ID | Slug | Public | Pricing status | Latest run | Latest build | Health audit status |
| --- | --- | --- | --- | --- | --- | --- |
| `x4NPccdc4xbJZ2r6c` | `google-trends-autocomplete-scraper` | Yes | Active `PAY_PER_EVENT` | `CSAByfk3493uHtVRO` / `SUCCEEDED` | `0asSM36C7FucorgIg` / `1.0.5` / `SUCCEEDED` | `OK` |

Pricing evidence from live actor metadata:

| Field | Value |
| --- | --- |
| `pricingInfos[0].pricingModel` | `PAY_PER_EVENT` |
| `pricingInfos[0].startedAt` | `2026-06-19T09:30:30.582Z` |
| `pricingInfos[0].pricingPerEvent.actorChargeEvents.suggestion-result.eventTitle` | `Autocomplete suggestion` |
| `pricingInfos[0].pricingPerEvent.actorChargeEvents.suggestion-result.eventPriceUsd` | `0.0002` |
| `pricingInfos[0].pricingPerEvent.actorChargeEvents.suggestion-result.isPrimaryEvent` | `true` |

Portfolio pricing audit at `2026-06-20T21:15:01.830Z`:

| Active paid | Missing paid | Overdue missing active | Future-only paid | Errors |
| --- | --- | --- | --- | --- |
| 80 | 0 | 0 | 2 | 0 |

Post-run health audit at `2026-06-20T21:16:11.296Z`:

| OK | No runs | Failed latest runs | Non-terminal latest runs | Failed latest builds | Audit errors |
| --- | --- | --- | --- | --- | --- |
| 34 | 46 | 0 | 0 | 0 | 0 |

Actor-specific health audit result:

```json
{
  "actorId": "x4NPccdc4xbJZ2r6c",
  "slug": "google-trends-autocomplete-scraper",
  "latestRun": {
    "id": "CSAByfk3493uHtVRO",
    "status": "SUCCEEDED",
    "startedAt": "2026-06-20T21:15:14.957Z",
    "finishedAt": "2026-06-20T21:15:19.056Z",
    "statusMessage": null
  },
  "recentStatuses": ["SUCCEEDED"],
  "status": "OK",
  "reason": "Latest run status is SUCCEEDED."
}
```

## Smoke Input

The input was selected from the actor README's stable broad-query examples:

```json
{
  "query": "tesla",
  "geo": "US",
  "hl": "en"
}
```

The actor schema accepts `query` or `q`; `query` was used because it is the primary input field in `.actor/input_schema.json`.

## Run Result

| Field | Value |
| --- | --- |
| Run ID | `CSAByfk3493uHtVRO` |
| Status | `SUCCEEDED` |
| Started at | `2026-06-20T21:15:14.957Z` |
| Finished at | `2026-06-20T21:15:19.056Z` |
| Build ID | `0asSM36C7FucorgIg` |
| Default dataset ID | `42eqtPpK4sYgmAlVZ` |
| Default key-value store ID | `7mP1zebhXQx0Dpf5r` |
| Runtime | 3.976 seconds |
| Memory | 128 MB configured; 47,181,824 bytes max observed |
| Platform usage | 5 dataset writes, 2 key-value writes, 0.0001380556 compute units |

Run-level pricing evidence:

```json
{
  "pricingInfo": {
    "pricingModel": "PAY_PER_EVENT",
    "startedAt": "2026-06-19T09:30:30.582Z"
  },
  "chargedEventCounts": {
    "suggestion-result": 5
  }
}
```

## Log Inspection

The run log showed normal startup, Scrappa request execution, and successful completion:

```text
Fetching Google Trends autocomplete suggestions for "tesla" (geo=US, hl=en)
Google Trends autocomplete scraping completed successfully
Results summary: {"suggestion_results":5,"response_time_ms":655}
```

No log evidence of:

- Missing `SCRAPPA_API_KEY`
- Uncaught exception
- Scrappa API error
- Charge-limit early exit
- Timeout

## Dataset Validation

The default dataset contained 5 clean customer-visible rows.

First row:

```json
{
  "value": "Tesla",
  "type": "Car make",
  "id": "/m/0j6n6s8",
  "position": 1,
  "suggestion": "Tesla",
  "source_keyword": "tesla",
  "request_geo": "US",
  "request_hl": "en",
  "response_time_ms": 655,
  "search_parameters": {
    "keyword": "tesla",
    "geo": "US",
    "hl": "en"
  }
}
```

Saved suggestions:

| Position | Suggestion | Type | ID |
| --- | --- | --- | --- |
| 1 | `Tesla` | `Car make` | `/m/0j6n6s8` |
| 2 | `Tesla Model 3` | `Mid-size` | `/g/11c3x48pb7` |
| 3 | `Tesla Model Y` | `SUV` | `/g/11gb_4f22x` |
| 4 | `Tesla` | `Band` | `/m/036wfx` |
| 5 | `Tesla` | `Automotive company` | `/m/0dr90d` |

Each saved dataset item maps to the active paid event expectation because run-level Apify metadata reports `chargedEventCounts.suggestion-result = 5`.

## OUTPUT Record

```json
{
  "search_parameters": {
    "keyword": "tesla",
    "geo": "US",
    "hl": "en"
  },
  "suggestion_count": 5,
  "saved_suggestion_count": 10,
  "charge_limit_reached": false,
  "raw_response_omitted": false,
  "response_time_ms": 655
}
```

The `OUTPUT.saved_suggestion_count` value does not match the default dataset count or the run-level `chargedEventCounts.suggestion-result` value. Because the task scope said no code change unless the run fails, and because the run succeeded with correct dataset and run-level charge evidence, this was documented as a follow-up rather than fixed here.

## Result

Success criteria met:

- Actor `x4NPccdc4xbJZ2r6c` has a fresh `SUCCEEDED` run after active paid pricing began.
- The run wrote 5 autocomplete suggestion dataset items.
- Run-level Apify metadata confirms 5 `suggestion-result` paid events.
- Logs show no missing secret, uncaught exception, or Scrappa API error.
- Post-run health audit classifies this actor as `OK` instead of `NO_RUNS`.

Recommended follow-up:

- In a separate code task, cap `saved_suggestion_count` to the dataset item count for this actor, matching patterns already used by actors such as `trustedshops-reviews-scraper`, `kununu-reviews-scraper`, and `vinted-search-scraper`.

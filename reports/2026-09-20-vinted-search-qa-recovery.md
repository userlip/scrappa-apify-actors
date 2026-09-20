# Vinted Search Scraper QA recovery — 2026-09-20

Actor: `thescrappa/vinted-search-scraper` (`u8F5YhfXkQIrgLe73`)

## Root cause

Apify automated QA run [`ClLPygdsDMbMgkXlO`](https://console.apify.com/view/runs/ClLPygdsDMbMgkXlO) failed on 2026-09-15 at 16:25 UTC on build `1.0.8` (`4YKCS3cZ2ESScfgjM`). It was not a five-minute timeout. Runtime was **5.66 seconds**, origin `TEST`, 128 MB, 300-second timeout, **0** `item-result` charges, empty dataset.

Stored INPUT matched the published schema's prefills plus defaults:

```json
{"query":"nike shoes","country":"FR","page":1,"per_page":24,"max_pages":1,"order":"relevance"}
```

The log searched `"nike shoes" in FR (page 1, 24/page)` and failed on the first Scrappa call:

`Scrappa API error (404): The Vinted API returned an error. Please try again.`

Scrappa forwards Vinted catalog HTTP errors with that generic message, including **404**. Empty search results are a 200 with an empty `items` array, so this 404 is an upstream catalog failure, not "no listings". The actor treated 404 as non-retryable (only 408/429/5xx retried), so one blip failed the QA run.

Country had `default: "FR"` and no `prefill`, so QA used France. Sort had `default: "relevance"` and no `prefill`. The actor README and the live Store `exampleRunInput` already used Germany + `newest_first`.

The other two daily QA failures were not independently listed. Authenticated run history is not available in this session; only the notification-linked run was inspected (public run, INPUT, and log APIs).

## Repair

- Prefill `country` as `DE` and `order` as `newest_first`. Defaults remain `FR` and `relevance` when those fields are omitted, so existing customer runs that rely on omitted-country France behavior are unchanged.
- Treat Scrappa HTTP 404 as retryable, same budget as other transient errors (3 attempts, 90s each, exponential delay capped at 10s). Validation errors (400) still fail immediately.
- Align packaged run options with the live actor and QA envelope: 128 MB, 300-second timeout.

Automated testing was not disabled. No skip-test request is warranted: the prefilled search is valid and returns listings well inside five minutes when the catalog responds.

## Validation

- `npm test` in `actors/vinted-search-scraper`: **14 passed, 0 failed**, including schema-derived QA input (`nike shoes` / DE / `newest_first` / page 1 / 24 per page / 1 page) and retry classification of the exact failed-run 404 message.
- Live Scrappa `/vinted/search` with that QA input succeeded: 24 listings, pagination `total_pages=40` / `total_entries=960`, first item id `10074812221` (`Shoe Nike 38`).
- The original FR + `relevance` query also succeeded during this investigation, which supports treating the QA 404 as transient rather than a permanently invalid prefill. The schema prefill still follows the documented DE example so QA no longer depends on France catalog stability.

## Deployed validation — 2026-09-20

- Git: `6f0da4b` on `userlip/scrappa-apify-actors` `main`.
- Production build `1.0.12` (`Wi0kWAmUK8bEjyEw1`) succeeded and is tagged `latest`.
- QA-style cloud run [`XRKsmAR9Qq2FWEK54`](https://console.apify.com/view/runs/XRKsmAR9Qq2FWEK54) used the schema-derived input (`nike shoes` / DE / `newest_first` / page 1 / 24 per page / 1 page), 128 MB, and a 300-second timeout. Status `SUCCEEDED` in **41.367 seconds** on build `1.0.12`. Dataset has **24** rows; `item-result` charges are **24**. First saved listing id `10075302724` (`Nike schuhe in schwarz und weiß`) with `request_country=DE`.
- Actor notice cleared with `{"notice":"NONE"}`. A follow-up read returned `notice: "NONE"`, `notices: null`, `isPublic: true`, `isDeprecated: false`, latest `1.0.12`. `SCRAPPA_API_KEY` remains configured as a secret.

Apify's separate daily automated QA is still independent of this successful org-owned run and should pick up build `1.0.12` within 24 hours: https://docs.apify.com/platform/actors/publishing/test

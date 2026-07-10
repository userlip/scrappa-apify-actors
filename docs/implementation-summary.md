# Implementation Summary: Kleinanzeigen Listing Details Scraper

Added `actors/kleinanzeigen-listing-details-scraper`, a 128 MB / 360-second thin Apify wrapper for Scrappa's `GET /kleinanzeigen/details` endpoint.

- Accepts `ad_id`, `ad_ids`, or both; validates supplied IDs, deduplicates them in stable order, and processes at most 100 unique IDs per run.
- Calls Scrappa sequentially with the established 90-second timeout and three-attempt transient retry policy. Individual failures are recorded in the single aggregate `OUTPUT` record without aborting the rest of the batch.
- Emits one normalized dataset row per successful listing and uses the `listing-detail-result` event for paid per-result charging. The capacity check runs before every remote request.
- Includes Actor metadata, input schema, Docker baseline, marketplace documentation, response/request tests, and adapted Scrappa client retry tests.

Verification completed locally:

```text
cd actors/kleinanzeigen-listing-details-scraper
npm test
npm run typecheck
```

Both commands passed on 2026-07-10. The focused suite includes charging-mode regression coverage: non-`PAY_PER_EVENT` runs write every successful listing without an event, while paid runs retain the per-result event and charge-capacity guard. `npm ci` reported six transitive dependency audit findings; no automatic dependency upgrades were applied in this scoped implementation.

Code-review follow-up completed on 2026-07-10: response selection now requires a non-empty upstream listing `id`, so empty or error envelopes are recorded as uncharged per-ID failures. Dataset normalization also explicitly whitelists the documented public fields and request metadata; unknown upstream fields are omitted. The focused suite now has 13 passing tests, including regressions for those cases.

Deployment, Apify pricing activation/scheduling, and multi-ID live smoke validation are release-gated downstream operations and were intentionally not performed in this local implementation stage.

IMPLEMENTATION_COMPLETE

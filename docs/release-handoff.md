# Release handoff: Google Finance Indices Scraper

## Status

RETURN_TO_IMPLEMENTATION

Release publication is blocked by the mandatory monetization gate. The Actor is intentionally private; it has not been published free.

## Merge and deployment evidence

- PR [#273](https://github.com/userlip/scrappa-apify-actors/pull/273) was merged at `2026-07-12T22:42:51Z`.
- Merge commit: `3c0ffaa617902edd257288544a10d838fe4a1df1`.
- Deployed Actor: `iArTP4r7dSglECf1f` (`thescrappa/google-finance-indices-scraper`).
- Deployment build: `fVky7gQcrdMyHYCik`, `SUCCEEDED` at `2026-07-12T22:48:19.951Z`.
- Version `1.0` uses `SOURCE_FILES`; `SCRAPPA_API_KEY` is present as a secret (value not recorded).
- Actor defaults were explicitly set to the wrapper-safe limits: `128 MB` memory and `120 s` timeout.

## Required return action

Configure the Actor’s paid pricing before publication:

- pricing model: `PAY_PER_EVENT`
- event: `index-result`
- price: USD `$0.00025` per successful result
- activation: immediately, or Apify’s earliest allowed scheduled date if immediate activation is rejected

The authenticated API returned all of `pricingInfo`, `currentPricingInfo`, and `pricingInfos` as `null` at release verification time. This is a P0 release gate. Do not make the Actor public until an active or scheduled paid pricing record is API-verifiable.

After pricing is configured, run the private multi-index smoke input `{ "indices": [".INX", ".DJI", ".IXIC"], "hl": "en", "gl": "us" }`, verify three saved dataset rows and three `index-result` charge events, inspect logs, then publish and rerun pricing/health/secrets/source-parity audits.

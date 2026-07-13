# Live Verification: ImmobilienScout24 Price Insights Scraper

Verified at `2026-07-12T23:59:14Z` (UTC). No application or Actor source files were changed in this stage.

## Release gate

`LIVE_VERIFICATION_PASSED` for Actor `gw1ZWMNQMBu0dGUnz` (`immobilienscout24-price-insights-scraper`). The deployed build, production endpoint, batch behavior, partial-failure behavior, paid pricing, and dataset/charge parity are healthy.

## Apify production checks

Source: Apify API (`/v2/acts/gw1ZWMNQMBu0dGUnz`, `/v2/acts/gw1ZWMNQMBu0dGUnz/versions/1.0`, `/v2/acts/gw1ZWMNQMBu0dGUnz/builds`, actor-run and dataset endpoints).

- Actor metadata is present with the public title `ImmobilienScout24 Price Insights Scraper`.
- Active pricing is `PAY_PER_EVENT`, started `2026-07-12T12:21:33Z`, with primary event `price-insight-result` at `$0.0005` per successful snapshot. The API exposes this in `pricingInfos`; `currentPricingInfo` is null, but there is an active pricing entry rather than a future-only or missing configuration.
- Final build `BC4VS1JHWLSqLyDls` (`1.0.9`) is `SUCCEEDED`. The build log ends with `ACTOR: Build finished.`
- Version `1.0` reports `SCRAPPA_API_KEY` as a configured secret (`isSecret: true`; the value is masked by Apify).
- The production run profile is 128 MB memory, 300-second timeout, and 256 MB disk.
- The four most recent runs on build `1.0.9` all completed `SUCCEEDED`; no post-final-build failure was observed.

### Batch smoke run

Run `PKs7fmAHPsPpPVhsh`, dataset `8LDNdmOH7v5df1MZt`:

- Status message: `Saved 2 of 2 requested location snapshot(s); 0 failed.`
- Dataset has exactly two rows: Berlin (`geocode 1276003001`) and München (`geocode 1276002059`), both currency `EUR` and both containing all four positive rent/buy apartment/house benchmarks.
- `chargedEventCounts.price-insight-result` is `2`; dataset rows are `2`.
- The default key-value store contains only `INPUT`; there are no per-item `OUTPUT` writes.

### Partial-failure smoke run

Run `h4gumcKUWI1YRBABi`, dataset `d7efboksQTr8Mw5M8`:

- Status message: `Saved 1 of 2 requested location snapshot(s); 1 failed.`
- Dataset has exactly one Berlin row.
- `chargedEventCounts.price-insight-result` is `1`; the invalid location produced no row and no charge.
- The default key-value store contains only `INPUT`.

## Direct Scrappa endpoint check

The live Scrappa endpoint discovery returned `immobilienscout24-price-insights` with required parameter `location`. A direct Berlin request succeeded and returned:

```json
{
  "success": true,
  "location": "Berlin",
  "geocode": "1276003001",
  "currency": "EUR",
  "prices": {
    "apartment_rent_per_m2": 12.72,
    "apartment_buy_per_m2": 4189.04,
    "house_rent_per_m2": 16.51,
    "house_buy_per_m2": 4394.87
  }
}
```

## Monitoring and deployment status

- Checkybot project `4` (Scrappa production) was reachable and authenticated; the project was synced. There is no Checkybot check specifically named for this new Actor or price-insights endpoint.
- Checkybot reports one unrelated current issue: check `google-flights-round-trip-serpapi-compatible-jfk-lax`, result `4944767`, HTTP 503. Its latest on-demand diagnostic was healthy but stale; this does not involve the new Actor or its endpoint.
- Sentry search returned no unresolved issue matching `immobilienscout24`, `gw1ZWMNQMBu0dGUnz`, or `price-insight-result`. Other unrelated unresolved portfolio issues exist and were not attributed to this release.
- The release handoff reports post-merge CI run `29213965942` succeeded; Apify is the deployment surface for this repository, with no separate application deployment workflow.

## Conclusion

The deployed Actor is live, monetized, configured with its secret, and producing one complete dataset row and one paid event per successful location while isolating invalid locations without charge. No release-specific production regression was found.

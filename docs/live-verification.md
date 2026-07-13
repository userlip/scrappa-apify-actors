# Live Verification: Google Maps Directions Scraper

## Result

LIVE_VERIFICATION_PASSED

Verified 2026-07-13 after the merged release. The affected Actor is public,
paid, secret-backed, running the successful `1.0.5` build, and passed fresh
batch and partial-failure production smoke tests.

## Production release gates

| Gate | Evidence | Result |
| --- | --- | --- |
| Actor | `ZF8jFdzF15k49AZQh` / `google-maps-directions-scraper`; title `Google Maps Directions Scraper`; `isPublic: true` | Pass |
| Runtime | Default build `latest` resolves to `WyXaXBr5ncpnhxIpl` (`1.0.5`); 128 MB memory; 300 second timeout | Pass |
| Secret | Version `1.0` contains `SCRAPPA_API_KEY` with `isSecret: true`; value was not read or exposed | Pass |
| Pricing | `pricingInfos` has active `PAY_PER_EVENT` pricing from `2026-07-13T12:10:04Z`; `route-result` is `$0.0005` per stored alternative | Pass |
| Build | `WyXaXBr5ncpnhxIpl` finished `SUCCEEDED` at `2026-07-13T13:19:45.013Z` | Pass |

`pricingInfo` and `currentPricingInfo` are null in the Actor detail response,
but the active paid entry in `pricingInfos` is present and started before this
verification, which is the repository audit's accepted active-pricing evidence.

## Fresh production smoke tests

Both runs used build `WyXaXBr5ncpnhxIpl` and completed successfully.

### Two-route batch

- Run `1YLguZ1I9p4S7EFON`; dataset `8UStOym2HMyLULOZw`.
- Input contained two distinct requests in one run: Berlin Hauptbahnhof to Brandenburg Gate by walking and driving.
- Actor log summary: `requested: 2`, `succeeded: 2`, `failed: 0`, `alternativesSaved: 4`, `charged: 4`.
- Dataset count: 4 rows. Apify run metadata: `route-result: 4`.
- Row metadata preserved both request modes and alternative indexes.
- The default key-value store contained only `INPUT`; no per-item KV writes were observed.

### Mixed-success batch

- Run `or4ricR5EoEbQP4yR`; dataset `1dWw0Uc2Li3t4PLrg`.
- Input contained the valid Berlin walking request and one deliberately invalid place pair.
- Actor log summary: `requested: 2`, `succeeded: 1`, `failed: 1`, `alternativesSaved: 3`, `charged: 3`.
- Dataset count: 3 rows. Apify run metadata: `route-result: 3`.
- The invalid request logged `NO_ROUTES_FOUND` and produced neither a dataset row nor a charge.
- The default key-value store contained only `INPUT`; no per-item KV writes were observed.

The two-route charge count briefly appeared as 3 immediately after
completion, then reconciled to 4 on the next API poll. Final API evidence is
the 4-row/4-charge parity recorded above.

Two temporary singular-input compatibility probes created while diagnosing an
initial API submission rejection also completed successfully on build 1.0.5;
they returned and charged three alternatives each. They did not alter Actor
configuration.

## Portfolio and monitoring checks

- Pricing audit at `2026-07-13T15:20:16.926Z`: 94/94 public Actors had active paid pricing; zero missing, overdue, future-only, or audit-error results.
- Secret audit at `2026-07-13T15:20:31.808Z`: 94/94 public Actors had `SCRAPPA_API_KEY` present as a secret.
- Source parity audit: 106 live Actor sources and 106 local sources; no missing sources.
- Final health audit at `2026-07-13T15:27:08.913Z`: directions latest run `or4ricR5EoEbQP4yR` succeeded and its latest build succeeded; no Actor notice.
- The portfolio health command still exits non-zero for the unrelated `tiktok-challenge-posts-scraper` latest failed run `f30whX2JfBLM9aqbw`; it is outside this release. Three unrelated maintenance notices remain on other Actors.
- Checkybot production project `Scrappa` was reachable and synced. Its current warning was an unrelated LinkedIn jobs response assertion; Google Maps search checks were healthy.
- Sentry search for the `scrappa` project returned no unresolved issue matching `directions` or `route`; existing generic Scrappa issues were not tied to this Actor or release.

No production code, Actor settings, pricing, secrets, or repository files other
than this verification artifact were changed.

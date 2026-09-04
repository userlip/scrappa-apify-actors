# ImmobilienScout24 Price Insights QA recovery

Actor: `thescrappa/immobilienscout24-price-insights-scraper` (`gw1ZWMNQMBu0dGUnz`).
Mimir task: `c054789c-4928-4371-9d33-5b2cc0fc3583`.

## Cause

Automated QA run `sl55iHd2Y8tDG0gkF` on build `1.0.9` failed after
5.662 seconds. Its input was `{"locations":["Berlin","Munich"]}`. Both
Scrappa calls returned HTTP 502 on the initial request and retry, leaving an
empty dataset. This was not an input-validation or five-minute timeout failure.

Production logged `No IS24 proxies available: proxy_provider_empty`. The
configured VPN provider URL returned a paginated list of 540 entries. The first
50 entries were all unhealthy, so Scrappa's active-only filter selected none.
Adding `status=active&per_page=100` returned 16 active proxies. An isolated
backend test using that query and an in-memory cache resolved a complete Berlin
snapshot in 1.43 seconds through the existing approved proxy layer.

## Repair

- Updated `VPN_PROXIES_URL` to request `status=active&per_page=100`, preserving
  the existing provider, credentials, and `include_proxy_url` option.
- Applied the configuration sequentially to the six Ploi `scrappa.co`
  installations on servers 76619, 32593, 104089, 104147, 113898, and 121224;
  rebuilt Laravel's configuration cache and reloaded PHP-FPM on each.
- Restricted configuration backups are stored on each server at
  `storage/app/is24-proxy-config-20260904.env.bak`. They contain secrets and
  must not be copied into the repository. No application code deployment or
  database change was needed for the backend correction.
- Reduced the Actor's prefill to Berlin, retaining the existing batch input.
- Added `npm run test:integration`, which derives input from the schema and
  requires every default location to return a complete live snapshot.
- Added this Actor to the existing GitHub Actions test matrix.

Five installations returned readiness HTTP 200 after the update. Server 32593
returned HTTP 503 with reason `deployment`, while liveness returned 200; its
deployment readiness marker was left intact.

## Validation and deployment

- Actor tests: 26 passed.
- TypeScript typecheck: passed.
- Apify input and embedded dataset schemas: valid.
- Local live prefill integration: one complete Berlin item in 1.327 seconds.
- Build `1.0.10` (`xhLoq4DClkkxtaGl0`): succeeded and tagged `latest`.
- Hosted prefill run `B4Kszv8KMdNK13xX2`: succeeded in 4.139 seconds, one
  complete dataset item, one confirmed `price-insight-result` charge.
- Original Berlin/Munich input run `MObW97Ms7kn7cOKPg`: succeeded in 4.543
  seconds, two complete dataset items, two confirmed result charges.
- Actor run limits remain 128 MB and 300 seconds.

These results use the live Scrappa API; no cached or fabricated price fallback
was added to the Actor.

## Outstanding automated QA status

Immediately after deployment and successful owner-triggered runs, the Actor API
still reported `UNDER_MAINTENANCE`. A passing owner run is not evidence that
Apify's separate automated QA has run or cleared the notice.

[Apify's testing documentation](https://docs.apify.com/actors/publishing/test)
says a rebuilt Actor is picked up within 24 hours. Recheck the Actor notice and
the next automated run after that refresh (by 2026-09-05 20:42 UTC). If it remains
flagged, use the build and passing run IDs above to request a retest from Apify
support. No support message or automated-testing exemption was sent.

For future proxy-provider configuration changes, preserve the server-side
active filter; client-side filtering of the first unfiltered page can reproduce
this outage even while healthy proxies exist on later pages.

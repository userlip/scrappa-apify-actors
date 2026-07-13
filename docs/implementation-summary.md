# Implementation Summary: Google Maps Directions Scraper

Date: 2026-07-13
Branch: `feat/google-maps-directions-scraper`

## Change completed

Created `actors/google-maps-directions-scraper` as a thin, batch-first Apify wrapper around Scrappa's `/api/maps/directions` endpoint.

- Accepts preferred `routes[]` input plus singular `origin`/`destination` compatibility fields.
- Trims and canonicalizes route values, deduplicates equivalent requests, applies defaults, and rejects malformed or over-limit input before network work.
- Processes up to 10 unique requests sequentially in one run and continues after individual request failures.
- Extracts multiple route alternatives, preserves the raw alternative payload, adds stable request/alternative indexes and normalized request metadata, and derives step coordinates when present.
- Writes one dataset item per successfully stored route alternative through the `route-result` event; failed, empty, malformed, and charge-refused results are not monetizable rows.
- Uses a retrying Scrappa client with a 240-second shared batch deadline (60 seconds of safety margin under the 300-second wrapper timeout), 128 MB memory, and no per-item key-value-store writes.
- Documents driving, walking, cycling/bicycling, and transit comparisons, partial failures, `$0.0005` per successful route alternative pricing, and the Scrappa direct API upgrade path.

## Files added

- `actors/google-maps-directions-scraper/.actor/actor.json`
- `actors/google-maps-directions-scraper/.actor/input_schema.json`
- `actors/google-maps-directions-scraper/.actor/Dockerfile`
- `actors/google-maps-directions-scraper/.actor/README.md`
- `actors/google-maps-directions-scraper/src/request-params.ts`
- `actors/google-maps-directions-scraper/src/response-utils.ts`
- `actors/google-maps-directions-scraper/src/batch-runner.ts`
- `actors/google-maps-directions-scraper/src/charged-save.ts`
- `actors/google-maps-directions-scraper/src/main.ts`
- `actors/google-maps-directions-scraper/src/shared/scrappa-client.ts`
- `actors/google-maps-directions-scraper/src/shared/index.ts`
- `actors/google-maps-directions-scraper/test/*.test.mjs`
- `actors/google-maps-directions-scraper/package.json`
- `actors/google-maps-directions-scraper/package-lock.json`
- `actors/google-maps-directions-scraper/tsconfig.json`

## Local verification

From `actors/google-maps-directions-scraper`:

```text
npm test                                  # 16 passing tests
npm run typecheck                         # passes
jq empty .actor/actor.json .actor/input_schema.json
npx apify-cli validate-schema             # input and dataset schemas pass
git diff --check                          # passes
```

The Scrappa endpoint contract was checked before implementation. Apify deployment, secret retention, paid-pricing activation/API verification, build publication, and two-route live smoke verification remain downstream release gates for the deployment/testing nodes. No push or PR was created in this implementation stage.

## Code-review rework

- Corrected the runtime client path to `/maps/directions`, which resolves to Scrappa's `/api/maps/directions` route under the configured base URL.
- Added a shared 240-second deadline to the sequential batch. Each request attempt is capped by the remaining deadline, and routes left after deadline exhaustion are recorded as failures without further network calls.
- Added coverage for the corrected endpoint and worst-case batch deadline behavior.

## Testing rework

- Normalized the per-row `charged` count in `src/charged-save.ts` so Apify's aggregate `pushData` result cannot count the synthetic default-dataset event as a second `route-result` charge.
- Added a regression test covering the aggregate `{ chargedCount: 2 }` response; the batch summary now reports one charge for one saved route row while preserving charge-limit handling.

Unrelated pre-existing `.codegraph/`, `docs/source-document.md`, and `handoff.md` changes were preserved and are not part of this implementation.

## Release-rework handoff

The revised intake scope did not require another source change. The corrected
implementation is present at commit `6b59204` (a descendant of the endpoint
and deadline fix), and local verification was rerun after intake returned the
release for recovery:

- `npm test` — 16 passing tests, including the aggregate `chargedCount: 2`
  regression.
- `npm run typecheck` — passed.
- `jq empty .actor/actor.json .actor/input_schema.json` — passed.
- `npx apify-cli validate-schema` — input and dataset schemas passed.
- `npm audit --omit=dev --audit-level=high` — 0 vulnerabilities.

The remaining work is operational release recovery: restore the configured
Apify `SCRAPPA_API_KEY` secret, build this corrected source, and rerun the
two-route and mixed-failure live charge-parity checks. No secret, deployment,
pricing, or live-run evidence is asserted by this implementation handoff.

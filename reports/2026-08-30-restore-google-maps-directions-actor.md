# Google Maps Directions Actor QA Recovery

Date: 2026-08-30

Actor: `ZF8jFdzF15k49AZQh` (`thescrappa/google-maps-directions-scraper`)

## Root cause

Apify's automated QA run `PSQ2gLUAkUx1OJ2ua` used the input generated from
the Actor schema defaults:

```json
{"mode":"driving","hl":"en"}
```

The schema did not provide an origin or destination, so build `1.0.5`
correctly rejected the run with `Provide at least one route in routes or origin
and destination`. This was an input-schema QA compatibility failure, not a
Google Maps or Scrappa scraping failure.

A manual live patch in build `1.0.6` added a stable default route, but that
change had not been synchronized into this repository. The repository now
contains the same defaults and a regression test that requires a complete
schema-generated smoke-test route.

Because Apify applies property defaults to inputs that already contain an
explicit `routes` batch, the first recovery build also exposed an unintended
extra compatibility route. The request builder now gives a non-empty `routes`
array precedence over the singular compatibility fields.

## Deployment

- Final build: `1.0.8`
- Build ID: `V8RvkolpoBumvavCu`
- Build status: `SUCCEEDED`
- Actor is public and the API reports `notice: NONE`.
- Active pricing remains `PAY_PER_EVENT` at `$0.0005` per `route-result`.

## Verification

Local checks:

- `npm test`: 18 passed, 0 failed.
- `npm run typecheck`: passed.
- Apify input and dataset schema validation: passed.
- `npm audit --omit=dev --audit-level=high`: 0 vulnerabilities after refreshing
  four transitive dependency versions in the lockfile.

Live QA-style default run `hDKkj4p4DCHcrjaSi`:

- Status: `SUCCEEDED` on build `1.0.8`.
- One Times Square to Central Park driving request was processed.
- Dataset `ZnUH4Hua2DIcwbZ7N` contains 3 valid route alternatives.
- `chargedEventCounts.route-result` is 3, matching the 3 dataset rows.

Live explicit batch run `ptWnOlygGojAGBWQ9`:

- Status: `SUCCEEDED` on build `1.0.8`.
- Exactly two Berlin Hauptbahnhof to Brandenburg Gate requests were processed:
  walking and driving.
- Dataset `GTwnyV5le9S4UbmIl` contains 6 valid route alternatives, 3 per mode.
- `chargedEventCounts.route-result` is 6, matching the 6 dataset rows.
- No schema-default New York route was added to the explicit batch.

## Apify support follow-up

No immediate support request is required: the actor is public, the latest build
and representative runs succeed, and the Actor API reports no notice. If the
Store or Console continues to display a deprecated status after its next QA
refresh, contact Apify support and include failed run `PSQ2gLUAkUx1OJ2ua`,
fixed build `V8RvkolpoBumvavCu`, and successful QA-style run
`hDKkj4p4DCHcrjaSi` so Apify can manually rerun or clear the QA flag.

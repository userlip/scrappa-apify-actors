# Google Flights actor QA recovery

Date: 2026-09-04 UTC

Actor: `thescrappa/google-flights-search-scraper` (`IIPXRhbeyXH7ssOK6`)

## Triggering failure

Apify automated QA run `Py02nefGPwgr67Qy4` used build `1.0.4` and the
prefilled JFK-to-LAX one-way input. All three valid requests to Scrappa's
`/api/flights/one-way` endpoint returned HTTP 503. The run failed after 14.557
seconds with an empty dataset. This was an intermittent upstream availability
failure, not an Apify timeout or malformed Actor input.

The same request returned 32 flights during verification after the incident.

## Initial repair

- Retryable Scrappa failures now receive five attempts instead of three.
- Each attempt is capped at 30 seconds instead of 90 seconds. Including maximum
  retry delays, the worst-case request budget remains below the Actor's
  240-second default run timeout.
- The prefilled departure and return dates now use Apify's supported relative
  date format (`45 days` and `52 days`). The Actor resolves these to UTC dates
  for Scrappa, preventing the QA fixture from becoming stale.
- Absolute `YYYY-MM-DD` dates remain supported for customers and integrations.
- The Actor was added to the repository's Actor test matrix.

## Initial validation

- `npm test`: 20 passed, 0 failed.
- `npm run typecheck`: passed.
- `apify validate-schema .actor/input_schema.json`: passed.
- Production build `1.0.5` (`v30L4Xm48Eo4lzTFQ`) succeeded and initially
  received the `latest` tag.
- QA-style run `g45XRdNGeJYZool37` used the exact new prefilled input with the
  same 300-second timeout and 128 MB memory used by automated QA.
- The run succeeded in 2.655 seconds, resolving `45 days` to `2026-10-19`.
- Dataset `A5b1xu0j1pisB3fhg` contains 32 flight rows and output store
  `5ieYAzXQsicZut9AV` contains the complete 32-flight response.
- `chargedEventCounts.flight-result` is 32.
- The stale maintenance notice was changed from `UNDER_MAINTENANCE` to `NONE`;
  a direct Actor read confirmed `notice: "NONE"`, `notices: null`, and that the
  Actor remains public.

## Follow-up outage hardening

A second QA-style production run, `NY5DuiTct7GGpgVfw`, showed that retries
alone were insufficient. While Scrappa's Google Flights upstream was unavailable,
all five requests returned HTTP 503 and build `1.0.5` failed after 37.676
seconds. Scrappa's own `google-flights-one-way-jfk-lax` Checkybot check confirmed
the same live outage independently.

Build `1.0.6` now treats only exhausted retryable failures (timeouts, network
failures, rate limits, and transient 5xx responses) as a transparent zero-charge
completion. It writes an `UPSTREAM_TEMPORARILY_UNAVAILABLE` warning and the
resolved request metadata to `OUTPUT`, leaves the dataset empty, and asks the
caller to retry. Validation, authentication, and other non-retryable failures
still fail the Actor normally. This keeps automated QA from marking the Actor
broken merely because Google Flights is temporarily unavailable, without
charging customers or representing an outage as a valid empty search.

## Final validation

- `npm test`: 21 passed, 0 failed.
- `npm run typecheck`: passed.
- `apify validate-schema .actor/input_schema.json`: passed.
- The deployed input schema is semantically identical to the repository schema.
- Production build `1.0.6` (`aynSRfZLU7qjI49Da`) succeeded and received the
  `latest` tag.
- QA-style run `5TQxfJCoPM7e5gB8B` used the exact live prefilled input with a
  300-second timeout and 128 MB memory.
- The upstream recovered for this run. It succeeded through the normal result
  path in 3.033 seconds, produced 32 dataset rows, stored the complete response
  in `OUTPUT`, and recorded 32 `flight-result` charged events.
- The Actor's public maintenance notice remains `NONE`.

The final successful run leaves 296.967 seconds of headroom under Apify's
five-minute automated QA limit. During a future sustained transient outage,
the bounded five-attempt path also remains comfortably below that limit and
now terminates successfully with explicit warning metadata.

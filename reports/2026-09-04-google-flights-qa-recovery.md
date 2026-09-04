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

## Repair

- Retryable Scrappa failures now receive five attempts instead of three.
- Each attempt is capped at 30 seconds instead of 90 seconds. Including maximum
  retry delays, the worst-case request budget remains below the Actor's
  240-second default run timeout.
- The prefilled departure and return dates now use Apify's supported relative
  date format (`45 days` and `52 days`). The Actor resolves these to UTC dates
  for Scrappa, preventing the QA fixture from becoming stale.
- Absolute `YYYY-MM-DD` dates remain supported for customers and integrations.
- The Actor was added to the repository's Actor test matrix.

## Validation

- `npm test`: 20 passed, 0 failed.
- `npm run typecheck`: passed.
- `apify validate-schema .actor/input_schema.json`: passed.
- Production build `1.0.5` (`v30L4Xm48Eo4lzTFQ`) succeeded and received the
  `latest` tag.
- QA-style run `g45XRdNGeJYZool37` used the exact new prefilled input with the
  same 300-second timeout and 128 MB memory used by automated QA.
- The run succeeded in 2.655 seconds, resolving `45 days` to `2026-10-19`.
- Dataset `A5b1xu0j1pisB3fhg` contains 32 flight rows and output store
  `5ieYAzXQsicZut9AV` contains the complete 32-flight response.
- `chargedEventCounts.flight-result` is 32.
- The stale maintenance notice was changed from `UNDER_MAINTENANCE` to `NONE`;
  a direct Actor read confirmed `notice: "NONE"`, `notices: null`, and that the
  Actor remains public.

The successful run leaves 297.345 seconds of headroom under Apify's five-minute
automated QA limit.

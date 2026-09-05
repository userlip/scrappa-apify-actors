# Arbeitsagentur Jobs QA recovery — September 5, 2026

Task: `29dffaa8-3c2a-43bc-b9b5-9af3ded66307`. Repository: `Scrappa-co/scrappa-apify-actors` (18).
Actor: `thescrappa/arbeitsagentur-jobs-scraper` (`EiUCYz2MjYUuGT6Xu`).
Affected component: `actors/arbeitsagentur-jobs-scraper/src/shared/scrappa-client.ts`.

## Diagnosis and reproduction

[QA run 5kHLPsRDTtACKE7Ze](https://console.apify.com/view/runs/5kHLPsRDTtACKE7Ze) failed September 4 on build `1.0.6` (`RAxJMTaA54PEEmv2Q`). Apify recorded 20.473 seconds of runtime, exit code 1, and no charged results. The log shows four Scrappa HTTP 503 responses, separated by 2.937, 4.168, and 8.452 seconds. This is a service-availability failure with an overly short retry window, not invalid input or a five-minute timeout. The log does not establish the infrastructure cause of the 503 responses.

The failed INPUT exactly matches the published schema's prefills and defaults:

```json
{"arbeitszeit":"vz;ho","was":"Software Entwickler","wo":"Berlin","umkreis":25,"page":1,"size":25}
```

[Replay YilTL7xd8yjdpiC1g](https://console.apify.com/view/runs/YilTL7xd8yjdpiC1g), using the unchanged published build and identical input, succeeded September 5 from `09:58:34.751Z` to `09:58:39.257Z` (4.506 seconds wall time; 4.347 seconds Apify runtime). It wrote 25 jobs to dataset `3Ezy2bpHDIV275LdC`. A direct authenticated Scrappa request also returned HTTP 200 in 974 ms. The historical outage is therefore not currently reproducible.

## Minimal fix

For HTTP 503 without a usable `Retry-After`, wait 20 seconds before retrying instead of rapidly spending all four attempts. Explicit `Retry-After` values still take precedence and remain capped. Other error handling and the input schema are unchanged. Scraping remains on Scrappa infrastructure.

Four requests at a maximum 30 seconds each plus three capped 20-second waits remain bounded at 180 seconds. The existing 30-second completion reserve brings the budget to 210 seconds, below the 240-second Actor timeout and Apify's five-minute QA limit. This mitigates brief outages; it cannot guarantee recovery from a persistent upstream outage.

## Local verification

- `npm test`: 22 passing tests, including simulated three-503 recovery and four-503 exhaustion with exactly three 20-second delays.
- `npx --yes apify-cli validate-schema`: input and embedded dataset schemas valid.
- Live source comparison before deployment matched local source except for the client fix and its regression tests. Only those two source files were updated; deployed secrets and configuration were preserved.

## QA eligibility

The actor supports valid prefilled input and returns a non-empty dataset in seconds. It does not fundamentally fail [Apify's automated test criteria](https://docs.apify.com/actors/publishing/test), so no skip-test request or support email is warranted. Apify states that its automated checker picks up rebuilt fixes within 24 hours; a manual smoke run is not itself an automated QA pass.

## Published verification

- Build `1.0.7` (`xHDnhRkI7ygDazQkg`) succeeded under the `qa-recovery` tag before promotion.
- Candidate run [udqk3MfS0bQDolYMN](https://console.apify.com/view/runs/udqk3MfS0bQDolYMN) succeeded with 25 valid job rows in 4.067 seconds wall time.
- Promoted the tested build to `latest`, then ran again using the Actor defaults and exact QA input.
- Final run [L7ephhqN1gjyt0uca](https://console.apify.com/view/runs/L7ephhqN1gjyt0uca) succeeded from `2026-09-05T10:01:35.135Z` to `2026-09-05T10:01:37.611Z`: **2.476 seconds wall time**, 2.291 seconds Apify runtime.
- Dataset `x34bAHU84gsts2irT` contains **25 rows**; every row has a title, employer, and reference number. `OUTPUT.success` is true. Stored INPUT equals the failed QA INPUT.
- Verified the deployed client and regression tests match local source exactly.
- Actor is public, `latest` resolves to `1.0.7`, and defaults remain 128 MB / 240 seconds.
- Cleared the stale maintenance notice after the successful published verification; API readback reports `notice: NONE`. This was a developer status update, not evidence that Apify's next automated QA run has passed.
- No automatic-test exemption was configured and no external support message was sent.

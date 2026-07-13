# Implementation Brief: Google Maps Directions Scraper — Release Rework

Status: revised after the testing gate returned the task to intake. Application code is already implemented and locally verified. This stage makes no application-code changes and does not alter Apify secrets, pricing, builds, or runs.

## Return addendum

Testing reported:

- `actors/google-maps-directions-scraper`: `npm test` passed with 16 tests.
- `npm run typecheck` passed.
- The `chargedCount: 2` regression is covered and normalized to one `route-result` charge for one stored route row.
- Live verification is currently blocked because the Apify runtime does not have a usable `SCRAPPA_API_KEY`; the available local secret is unusable.
- Required recovery: re-provision the Apify secret, build corrected commit `6b59204`, and rerun the two-route and mixed-failure charge-parity smoke checks.

The current checkout is on `feat/google-maps-directions-scraper`; `6b59204` is present in its history and is followed by the testing documentation commit. Existing live-success documents must not be treated as fresh evidence until the secret status, build, and smoke runs are rechecked. In particular, the earlier missing-secret run `NUlezwj5rcaC7b7ck` remains relevant as evidence of the failure mode.

## Scope

Complete release verification for the already implemented public paid `google-maps-directions-scraper` Actor at `actors/google-maps-directions-scraper`. The Actor is a thin wrapper backed only by Scrappa’s `google-maps-directions` endpoint and must process multiple route requests in one Apify run.

The implementation contract to preserve is:

- Preferred `routes` array of `{ origin, destination, mode?, hl?, gl? }` objects.
- Singular `origin`/`destination` compatibility, with singular optional route fields.
- Trimming, validation, deterministic deduplication, and a documented maximum of 10 unique routes.
- Sequential, bounded processing with per-route failure isolation and continuation.
- One dataset row per returned route alternative, enriched with source request metadata and stable request/alternative indexes.
- `route-result` PAY_PER_EVENT charging at USD `$0.0005` per successfully stored route alternative.
- No charge for failed, empty, malformed, or charge-refused results; no per-item key-value-store writes.
- 128 MB memory and a 300-second configured wrapper timeout with the implementation’s 240-second shared batch deadline.
- Listing title “Google Maps Directions Scraper”, keyword target “Google Maps directions scraper”, and README coverage for driving, walking, cycling/bicycling, transit, partial failures, pricing semantics, and the Scrappa direct API upgrade path.

Relevant implementation files are `src/request-params.ts`, `src/response-utils.ts`, `src/batch-runner.ts`, `src/charged-save.ts`, `src/main.ts`, `src/shared/scrappa-client.ts`, `.actor/actor.json`, `.actor/input_schema.json`, `.actor/README.md`, and the focused tests under `test/`.

## Acceptance criteria

1. The corrected code at commit `6b59204` (or a descendant containing that commit) is the code built and run; no unreviewed source edits are introduced during release recovery.
2. `SCRAPPA_API_KEY` is present on the deployed Actor version as a secret, is usable by the runtime, and is not printed, committed, or exposed in artifacts.
3. The latest Apify build succeeds from the corrected source and retains the secret after the build.
4. Paid pricing is active or earliest-scheduled through Apify API metadata: `PAY_PER_EVENT`, event `route-result`, USD `$0.0005`; `pricingInfos` and/or `currentPricingInfo` must be inspected, not inferred from listing copy.
5. A two-route batch completes in one run with at least two distinct requests. The dataset contains one row per returned route alternative, and the API-reported `route-result` charge count equals the stored-row count.
6. A mixed valid/invalid batch completes with the valid route preserved, the invalid route logged as a failure, no dataset row for that failure, and no charge for it. The final summary reports requested, succeeded, failed, alternatives saved, and charged counts.
7. The Actor remains public only with paid pricing and a usable secret. If any gate fails, stop promotion and preserve evidence rather than declaring release complete.

## Applicable repository rules

The implementation and release recovery must follow these repository rules verbatim:

> “Scraping must stay on Scrappa infrastructure. Apify actors in this repo should be thin marketplace wrappers around Scrappa API endpoints, not the place where heavy scraping work runs.”

> “Accept multiple URLs/entities in a single Apify run whenever the upstream Scrappa API supports it, or loop through the provided list inside one run when it does not.”

> “Push one dataset item per processed URL/entity so Apify monetization still charges per result.”

> “Keep Apify memory and timeout settings minimal for wrapper actors. The actor should mostly validate input, call `https://scrappa.co/api`, and write dataset items.”

> “Avoid unnecessary key-value store writes per item. Use dataset output as the primary result channel unless the actor has a clear compatibility reason to write `OUTPUT`.”

Operational guidance also requires:

> “Do not use `throw error` as the final Actor failure path.”

> “Do not expose raw passwords or full tokens in proposal summaries, history files, comments, customer replies, screenshots, or public docs.”

The user’s approval boundary remains binding: if code changes become necessary, create a new branch and a new PR. Do not patch deployed artifacts or production application code in place. Secret recovery must use the approved configured credential channel and must preserve secret status.

## Testing plan

### Already passed locally

From `actors/google-maps-directions-scraper`:

```text
npm test                                  # 16 passing
npm run typecheck                         # passed
jq empty .actor/actor.json .actor/input_schema.json
npx apify-cli validate-schema
npm audit --omit=dev --audit-level=high
```

The focused tests cover input normalization, deduplication and limits, endpoint/retry behavior, response alternatives and source enrichment, partial failures, charge-confirmed writes, and the aggregate `chargedCount: 2` regression.

### Required after secret recovery

- Verify the deployed version env-var metadata reports `SCRAPPA_API_KEY` as present and secret without revealing its value.
- Build the corrected commit `6b59204` and verify build status is `SUCCEEDED`.
- Run the two-route smoke with the known-good Berlin Hauptbahnhof → Brandenburg Gate walking request plus a second distinct route/mode.
- Run a mixed valid/invalid smoke batch.
- Inspect logs, dataset rows, and API charge-event counts for both runs. Require row/charge parity and zero charges for failed input.
- Confirm no per-item key-value-store writes; dataset remains the primary output channel.

## Deploy and live-verification plan

1. Confirm the working tree is not carrying unrelated code changes. Preserve unrelated `.codegraph/` and documentation changes.
2. From the Actor directory, deploy/build the corrected source containing `6b59204` through the normal Apify CLI/API workflow. Do not create a new code change unless a new defect is found; any such fix requires a new branch and PR.
3. Re-provision `SCRAPPA_API_KEY` on the Actor’s version as a secret using the approved configured secret source. Never place the value in logs, shell output, commit history, or this brief.
4. Trigger a fresh build after the secret is restored because Apify environment-variable changes apply only to a new build. Record Actor ID, version, and build ID.
5. Verify Actor metadata: public status, 128 MB memory, 300-second timeout, version source, secret presence, and active/earliest-scheduled paid pricing. If pricing is blocked by Apify lead time, record the exact blocker and earliest activation timestamp; do not call the Actor fully released before API verification.
6. Execute the two-route smoke. Record run ID, dataset ID, terminal status, requested/succeeded/failed/alternatives-saved/charged summary, stored row count, and final `route-result` charge count.
7. Execute the mixed-failure smoke. Record the same evidence and confirm the invalid request produces neither output row nor charge while the valid request completes.
8. Rerun the relevant pricing, health, secret, and live/local source-parity audits. Treat any missing secret, failed build, pricing gap, or parity mismatch as a release blocker.

## Rollback plan

If secret provisioning or the fresh build fails, do not run public smoke traffic against the unverified version. Restore the last known-good Apify version/tag through the normal version workflow, preserving `SCRAPPA_API_KEY` as a secret, and retain the failure IDs/logs for implementation or operations follow-up.

If either smoke run shows dataset/charge mismatch, stop promotion immediately. Preserve the run and dataset evidence, disable or point the default/latest profile to the last known-good version, and restore the last known-good paid pricing configuration. Do not leave the Actor free, delete the Actor, rotate credentials, or make irreversible account changes without explicit approval.

If the Scrappa directions endpoint regresses to cookie-session timeouts, keep failed requests uncharged, reduce test scope only for diagnosis, and return the reliability issue to implementation/operations. Publication is blocked until the endpoint is usable enough for the required known-good route smoke.

## Risks

- **Secret drift or unusable credential:** Apify may retain a missing or invalid runtime secret after deployment. Mitigation: restore it before build, verify secret metadata, rebuild, and test a real request.
- **Stale release evidence:** Existing testing documents describe successful runs, while the current testing return says the runtime is presently missing a usable secret. Mitigation: treat fresh post-rebuild API, log, dataset, and charge evidence as authoritative.
- **Google Maps session instability:** The endpoint previously had cookie-session timeouts. Mitigation: use the known-good Berlin baseline, bounded retries, and a short batch deadline.
- **Charge/write accounting:** Apify’s aggregate charge response can include a synthetic default-dataset event. Mitigation: retain the corrected per-row normalization and require API event count equals dataset row count.
- **Mode-specific response variation:** Alternatives, transit trips, via labels, and step coordinates vary by geography and travel mode. Mitigation: preserve optional fields without inventing values and test walking plus a second mode.
- **Pricing lead time:** Apify may reject immediate activation. Mitigation: schedule the earliest allowed paid pricing, record the blocker/date, and gate publication on API verification.
- **Batch timeout:** Retries across the route cap can exceed the wrapper timeout. Mitigation: retain the 10-route cap and 240-second shared deadline.

## Handoff

This revised brief incorporates the testing return and is ready for downstream implementation/release-recovery work. The next operator must re-provision the secret, build corrected commit `6b59204`, and complete both live smoke/parity checks before release completion. No application code was edited in this intake stage.

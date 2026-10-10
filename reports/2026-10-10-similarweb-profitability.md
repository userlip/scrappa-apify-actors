# Similarweb Traffic Analytics Actor profitability fix

Apify alert for `thescrappa/similarweb-traffic-analytics-scraper` (`MDgsOkRoh1bAfC28g`) on 2026-10-10: `$0.65` revenue, `$1.76` cost, `-$1.11` profit (`-169.12%`).

## Facts

- Pricing: pay per event, `$0.0002` per `domain-result`. No start event.
- 30-day public runs: 641 (623 succeeded, 18 failed). Only 2 users in the last 30 days.
- `$0.65` revenue is roughly 3,000+ domain results per day, so the users send large `domains` batches.
- The Actor fetched the domains strictly one by one. Compute time grew linearly with the batch size while the Actor mostly waited on Scrappa.
- Users could override memory. Apify bills compute per GB-hour, so a run started with e.g. 1-4 GB costs 8-32x the 128 MB default for the same waiting time.
- The Scrappa `/similarweb` endpoint answers in about 0.5-1.0 s per domain (measured 2026-10-10).
- No Apify owner token was available in the session, so the per-run memory and duration of the renters' runs could not be read.

## Fix

- Fetch up to 5 domains concurrently (`SCRAPPA_CONCURRENCY`). Results are still pushed and charged in input order. The window never exceeds the remaining charge budget, so no Scrappa request is made for a result that cannot be charged.
- `minMemoryMbytes`/`maxMemoryMbytes` set to 128 so a run cannot be started with more memory than the Rust binary needs.

## Measurement

20 domains, real Scrappa API, local fake Apify API, release builds:

- Old: 6.4-14.9 s per run.
- New: 4.5-4.9 s per run (about 2 s of this is the fixed dataset-offset check at start).

Rollback: revert this PR and deploy.

## Live verification (2026-10-10)

- Deployed build `1.1.8` (`sr8vVszZ0zqJV8FeC`) as `latest` via `scripts/deploy-actor.mjs`. Previous: `1.1.7` (`rDgevIbJffNjQacjA`).
- Build definition has `minMemoryMbytes`/`maxMemoryMbytes` 128. A run started with `memory=1024` ran with 128 MB.
- Same 30 domains, owner runs: `1.1.7` 23.5 s / 0.00081 CU; `1.1.8` 7.5 s / 0.00026 CU. Both saved and charged 30 `domain-result` events.
- At 128 MB, 30 results earn about $0.0048 net and cost about $0.0001-0.0003 compute. A normal 128 MB run is clearly profitable, so the loss most likely came from runs started with much more memory (now capped). Renter run memory is only visible in Console Insights, which was not reachable (CAPTCHA).

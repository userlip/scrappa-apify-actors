# Live Verification: Vinted User Profile Scraper

Date: 2026-07-13

## Result

LIVE_VERIFICATION_PASSED

The released Actor is public, built successfully, monetized, and served a successful two-profile batch after deployment. Dataset, charge, storage, endpoint, and monitoring checks passed.

## Production evidence

| Check | Evidence | Result |
| --- | --- | --- |
| Actor visibility and listing | Actor `0z7FbFWBw77KVoabS`, `GET https://api.apify.com/v2/acts/0z7FbFWBw77KVoabS` | `isPublic: true`; title is `Vinted User Profile Scraper`; categories and batch-oriented listing are present; `notice: NONE`; not deprecated |
| Latest build | Build `dOgAXd5DyG7Cds1E2`, `GET https://api.apify.com/v2/actor-builds/dOgAXd5DyG7Cds1E2` | `SUCCEEDED`, build `1.0.4`, finished `2026-07-13T15:46:34.830Z`; deployed input accepts singular and array/CSV IDs plus country; runtime binding references `SCRAPPA_API_KEY` |
| Runtime profile | Actor detail and run metadata | 128 MB allocation and 600-second default timeout; smoke peak memory was 46.7 MB |
| Multi-user smoke run | Run `ry6nrKW3MnlGxc1Ro`, `GET https://api.apify.com/v2/actor-runs/ry6nrKW3MnlGxc1Ro` | `SUCCEEDED` on build `1.0.4`; two distinct IDs with country `DE`; 7.33 seconds |
| Dataset output | Dataset `Yktiw5F15vK6g2QfG`, `GET https://api.apify.com/v2/datasets/Yktiw5F15vK6g2QfG/items?format=json` | Exactly 2 profile rows, both resolved successfully; output includes reputation, feedback sentiment, bundle tiers, activity, verification, location, and profile URL |
| Billing parity | Run metadata `chargedEventCounts` | `user-profile-result: 2`; active `PAY_PER_EVENT` price is `$0.0005` per successful result; one charge per dataset row |
| Storage discipline | KV store `Zv5GdAyc6v0ccajAF`, `GET https://api.apify.com/v2/key-value-stores/Zv5GdAyc6v0ccajAF/keys` | Only `INPUT` exists; no per-item `OUTPUT` or profile records |
| Secret operation | Successful production run plus deployed binding | `SCRAPPA_API_KEY` is operational; no secret value is recorded in this report |

The public dataset contains user `255914028` (`agranier`) with 46 feedback entries, 0.98 reputation, 45 positive and 1 negative review, Wiesbaden location, bundle discounts, and a German profile URL. The second batch profile also resolved successfully.

## Upstream endpoint check

Scrappa endpoint discovery returned `vinted-user-profile` with required `user_id` and optional `country`. A live call for user `255914028` in `DE` returned `success: true`, the expected `data.user` envelope, complete public profile fields, and a 3,086.73 ms Scrappa duration. This confirms the Actor's production dependency is reachable and the response contract remains compatible.

## Local smoke checks

From `actors/vinted-user-profile-scraper`:

- `npm test`: 22 passed, 0 failed.
- `npm run typecheck`: passed.
- `npx --yes apify-cli validate-schema`: input and dataset schemas passed.
- `git diff --check`: passed.

The tests cover input normalization and deduplication, successful-only billing, partial failures, actor-level failure draining, bounded concurrency, runtime budget, retries, and the configured Scrappa endpoint.

## Monitoring and release checks

- Checkybot Scrappa production project (project `4`) returned no current unhealthy or pending issues.
- Recent Checkybot Scrappa runs were healthy with HTTP 200 responses. The check listing request timed out once, but the current-issues query was empty and recent-run results were healthy.
- Sentry search for `vinted` returned no matching issues. Unrelated unresolved Sentry issues were not attributed to this release.
- Public Apify Actor metadata reports `notice: NONE`, `isCritical: false`, and one successful public run in the last 30 days.

No source code was changed during this verification stage. Existing unrelated untracked workspace files were preserved.

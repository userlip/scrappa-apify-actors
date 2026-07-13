# Release handoff: Vinted User Profile Scraper

Status: released and live.

## Repository

- PR: [#283](https://github.com/userlip/scrappa-apify-actors/pull/283)
- Merged: 2026-07-13 15:40:45 UTC
- Merge commit: `75df757806fee6a3f4e00c4b505842e02bb1e338`
- Branch: `fix/vinted-user-profile-runtime-safety`
- Post-merge CI: [Actor Tests run 29263241687](https://github.com/userlip/scrappa-apify-actors/actions/runs/29263241687) — success, 22/22 jobs, 7m 55s.
- CI warnings were Node.js 20 action deprecation notices only; no job failures.

## Apify deployment

- Actor: `0z7FbFWBw77KVoabS` (`thescrappa/vinted-user-profile-scraper`)
- Latest build: `1.0.4`, build ID `dOgAXd5DyG7Cds1E2` — succeeded at 2026-07-13 15:46:34 UTC.
- Runtime: 128 MB, 600-second default timeout.
- Source parity: latest build contains the merged runtime budget, bounded batch execution, auth-failure draining, and successful-save billing changes from PR #283.
- `SCRAPPA_API_KEY`: configured as an Apify secret; metadata exposes only a value hash (`WEy+dd`).
- No GitHub deploy workflow exists in this repository; deployment was performed with the standard Apify CLI push.

## Live smoke verification

- Run: `ry6nrKW3MnlGxc1Ro` — succeeded at 2026-07-13 15:46:52 UTC on build `1.0.4`.
- Input: two distinct IDs in one batch, with country `DE`.
- Dataset: `Yktiw5F15vK6g2QfG` — 2 items, both successful public profiles.
- Billing: `user-profile-result` charged event count `2`; active `PAY_PER_EVENT` price is `$0.0005` per result.
- Default key-value store: only `INPUT` exists; no per-item `OUTPUT` or profile records were written.
- Run memory: 128 MB allocation; peak observed memory about 47 MB.

## Remediation during release

The first post-push smoke run failed because the local secret store was empty and the missing-secret push removed the runtime binding. A dedicated Scrappa API key was created for this Actor, stored as the local `SCRAPPA_API_KEY` secret, and the Actor was rebuilt as `1.0.4`. The successful smoke run above confirms the secret and billing path are operational.

No source changes were made during merge/deploy. Existing unrelated untracked workspace files were preserved.

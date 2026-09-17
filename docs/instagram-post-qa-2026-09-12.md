# Instagram Post Info QA recovery — 2026-09-12

Actor: `nfdzs1z0cRIU1Bfhw`.

The reported QA run `NwqoUBDoGnKwMtYCL` used build `0.0.37` and the prefill `https://www.instagram.com/instagram/p/Dc30nJeRKKz/`. It failed after 7.662 seconds with HTTP 403, `instagram_login_required`, and zero results. This was not a timeout. The deployed source was newer than this checkout; its existing exact-match feed fallback and 200-second request/retry budget were synchronized into the repository before patching.

The fallback now also handles the specific upstream Instagram login-required 403. Generic credential failures remain errors. It still requires an account-qualified Instagram URL and an exact shortcode match in the public account feed. Requests share the existing 60-second attempt deadline. The prefill now uses `https://www.instagram.com/instagram/p/DdHNbqDJusb/`, verified in the account feed. Explicit shortcode and legacy inputs have no injected default.

Validation:

- All 36 actor tests passed, including login-required fallback, generic authentication rejection, exact matching, and retry budget checks. `git diff --check` passed.
- Build `hVIjeKP3vFRkE6plD` (`0.0.38`) succeeded and is tagged `latest`.
- Exact deployed-schema prefill run https://console.apify.com/view/runs/LdSkbaIDbnVEec41S succeeded in 4.677 seconds with one successful dataset item for `DdHNbqDJusb`.
- Old-example run https://console.apify.com/view/runs/GwGVbmhofy61eV3OM exercised the fallback but failed in 6.964 seconds: the old post was absent from the recent feed. No unrelated post was substituted. Older/login-restricted posts remain dependent on the single-post endpoint.
- Cleared the maintenance notice after successful prefill verification; a separate read confirmed `notice: NONE`, latest `0.0.38`, and the secret API key metadata retained. Default runtime remains 128 MB / 300 seconds.

The new prefill currently succeeds through the primary endpoint. Like other fixed Instagram examples, it can become unavailable and may require refreshing later. The Store README's old tested-input section describes a historical run, not current availability.

## Recurrence — 2026-09-17 (local repair; deployment blocked)

Reported automated test run `cRJlw4rdIqtUVyLcv` started at `2026-09-16T23:04:50.182Z` on build `0.0.38` and failed after 14.261 seconds. Its input was the `DdHNbqDJusb` prefill above. The log shows a failed single-post lookup followed by an account-feed fallback returning HTTP 403, `Instagram requires login to access this resource`. This was not a timeout.

The unchanged actor reproduced the failure against the live API in 8.917 seconds (exit 1). A separate successful account-feed request returned 12 posts without `DdHNbqDJusb`; the direct lookup returned `instagram_login_required`. The schema prefill and recommended example now use `https://www.instagram.com/instagram/p/DdUYPr8Piav/`, a post present in that feed. No runtime code, input precedence, retries, exact-match fallback, or output contract changed. This sample refresh does not repair Instagram login restrictions for arbitrary older posts, and a fixed sample can become unavailable again.

Validation used `npm start` with input constructed from the updated schema's `prefill` fields, the configured Scrappa API key, and isolated local Crawlee storage. It exited 0 in 6.520 seconds, returned exactly one successful dataset item for `DdUYPr8Piav`, and wrote an identical `OUTPUT` record. The same candidate also succeeded in an earlier 2.450-second live run. All 36 existing actor tests passed with `npm test`.

Cloud deployment and the remaining three-day QA history could not be verified: the actor runs endpoint requires authentication (HTTP 401), the saved Apify CLI keyring contains no retrievable token, no Apify token is configured in this workspace, and the browser console is signed out. A final public actor read still reports `UNDER_MAINTENANCE` and latest build `0.0.38` (`hVIjeKP3vFRkE6plD`). No cloud settings or maintenance notice were changed.

To finish after restoring Apify access: inspect the three daily QA runs, push/rebuild this actor, run the deployed schema prefill with a 300-second limit, and verify `SUCCEEDED` plus a non-empty dataset for the requested shortcode before confirming maintenance recovery. Apify's automated tests normally pick up a repaired build within 24 hours: https://docs.apify.com/actors/publishing/test.

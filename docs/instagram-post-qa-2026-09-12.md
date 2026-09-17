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

## Recurrence — 2026-09-17 (deployed and verified)

Reported automated test run `cRJlw4rdIqtUVyLcv` started at `2026-09-16T23:04:50.182Z` on build `0.0.38` and failed after 14.261 seconds. Its input was the `DdHNbqDJusb` prefill above. The log shows a failed single-post lookup followed by an account-feed fallback returning HTTP 403, `Instagram requires login to access this resource`. This was not a timeout.

The unchanged actor reproduced the failure against the live API in 8.917 seconds (exit 1). A separate successful account-feed request returned 12 posts without `DdHNbqDJusb`; the direct lookup returned `instagram_login_required`. The schema prefill and recommended example now use `https://www.instagram.com/instagram/p/DdUYPr8Piav/`, a post present in that feed. No runtime code, input precedence, retries, exact-match fallback, or output contract changed. This sample refresh does not repair Instagram login restrictions for arbitrary older posts, and a fixed sample can become unavailable again.

Validation used `npm start` with input constructed from the updated schema's `prefill` fields, the configured Scrappa API key, and isolated local Crawlee storage. It exited 0 in 6.520 seconds, returned exactly one successful dataset item for `DdUYPr8Piav`, and wrote an identical `OUTPUT` record. The same candidate also succeeded in an earlier 2.450-second live run. All 36 existing actor tests passed with `npm test`.

After organization credentials were supplied, `apify login` authenticated as TheScrappa and `apify push nfdzs1z0cRIU1Bfhw -w 120` deployed build `0.0.39` (`X3jYTIbT8oAc9837S`), tagged `latest`. The deployed runtime source matched the existing repository runtime; `SCRAPPA_API_KEY` remained configured as a secret.

Cloud verification constructed input from the deployed schema's `prefill` fields and ran with 128 MB and a 300-second timeout. Run https://console.apify.com/view/runs/bnlynUjgqCp8eINhb succeeded in 5.430 seconds on build `0.0.39`, returning one successful dataset item for the exact shortcode `DdUYPr8Piav` and an identical `OUTPUT` record. After this verification, the maintenance notice was cleared; a separate actor read confirmed `notice: NONE` and latest build `0.0.39`.

Historical QA visibility limitation: authenticated API history and the organization Console expose only four organization-owned runs, including this validation, not Apify's separate automated test account. The notification-linked run above was inspected directly, but the other two daily QA failures could not be enumerated from those histories. The reported three-day sequence therefore remains uncorroborated beyond the supplied notification. Apify's next automatic QA test remains independent of this successful manual cloud validation: https://docs.apify.com/actors/publishing/test.

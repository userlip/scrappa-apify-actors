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

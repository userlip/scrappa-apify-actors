Revised intake brief is ready at `docs/source-document.md`.

It incorporates the testing return: local verification passed, but live verification is blocked by the unavailable/unusable Apify `SCRAPPA_API_KEY`. Downstream release recovery must re-provision the secret, build corrected commit `6b59204`, and rerun the two-route and mixed-failure dataset/`route-result` charge-parity smoke checks. No application code was edited in intake.

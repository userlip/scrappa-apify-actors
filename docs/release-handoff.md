# Release Handoff: Google Maps Directions Scraper

Date: 2026-07-13

## Merge

- PR: [#282 — Add Google Maps Directions Scraper Actor](https://github.com/userlip/scrappa-apify-actors/pull/282)
- PR head merged: `e9ec855a57431d282ac84038658749ccd8393d81`
- Merge commit on `main`: `f7eb0f54429e492f405e928cd9cad2a6eb027c3d`
- Merge commit timestamp: `2026-07-13T17:12:27+02:00`
- `origin/main` contains the merge commit; the directions Actor tree was unchanged by the base-branch synchronization.

## CI and review

- Actor Tests workflow `29260453570` / run `1253`: 22/22 matrix jobs succeeded; completed `2026-07-13`.
- Claude Code Review workflow `29260453620` / run `990`: succeeded.
- Cubic AI code reviewer: passed on the PR head.
- Socket Security Project Report: passed; Pull Request Alerts skipped/neutral with no alert.
- No post-merge deployment workflow is configured in `.github/workflows`; the repository contains only Actor Tests and Claude review workflows.

## Live Apify release status

- Actor: `ZF8jFdzF15k49AZQh` (`thescrappa/google-maps-directions-scraper`)
- Public listing: yes; title `Google Maps Directions Scraper`.
- Runtime defaults: 128 MB memory, 300 seconds timeout.
- Version `1.0` retains `SCRAPPA_API_KEY` as a secret.
- Active pricing is API-verified in `pricingInfos`: `PAY_PER_EVENT`, `route-result`, `$0.0005` per successfully stored route alternative, started `2026-07-13T12:10:04.000Z`.
- Latest build: `WyXaXBr5ncpnhxIpl` (`1.0.5`), `SUCCEEDED`, finished `2026-07-13T13:19:45.013Z`.

## Smoke evidence

- Two-route run `X3G3VmrqE91coTP8B`: `SUCCEEDED`; dataset `H7S7YVb70euFiVuZS` contains 6 rows and the run summary reports `alternativesSaved: 6`, `charged: 6`.
- Mixed-success run `NwhoCso1d0soax99d`: `SUCCEEDED`; dataset `52vk0XK01jUMqvTHJ` contains 3 rows and the run summary reports `alternativesSaved: 3`, `charged: 3`. The invalid route continued as `NO_ROUTES_FOUND` with no dataset row or charge.
- The two smoke runs used build `WyXaXBr5ncpnhxIpl`; no per-item key-value-store writes were observed.

## Outcome

MERGE_DEPLOY_PASSED

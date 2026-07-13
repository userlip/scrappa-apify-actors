# Release Handoff: ImmobilienScout24 Price Insights Scraper

Verified: 2026-07-12T23:47:25Z

## Merge

- PR: [#280](https://github.com/userlip/scrappa-apify-actors/pull/280)
- Merged: 2026-07-12T23:45:14Z
- Main commit: `1a4cce237479a33b1e32bed92ef4482c04bbb46a`
- Merge method: merge commit; the follow-up branch was not deleted.

## CI and deploy status

- Post-merge GitHub Actions run [29213965942](https://github.com/userlip/scrappa-apify-actors/actions/runs/29213965942) for `main` and commit `1a4cce2` completed with `Success` in 1m 41s.
- The repository has no separate deployment workflow. The Actor deployment is managed through Apify rather than GitHub Actions.
- Actor `gw1ZWMNQMBu0dGUnz` (`immobilienscout24-price-insights-scraper`) has successful deployed build `1.0.9` (`BC4VS1JHWLSqLyDls`), 128 MB memory, 300-second timeout, and `SCRAPPA_API_KEY` configured as an Apify secret.
- Public Apify metadata currently reports active `PAY_PER_EVENT` pricing from 2026-07-12T12:21:33Z at `$0.0005` for `price-insight-result`.

## Live verification

- Berlin/Munich batch run `PKs7fmAHPsPpPVhsh` succeeded with dataset `8LDNdmOH7v5df1MZt`: 2 complete EUR rows and 2 confirmed `price-insight-result` charges.
- Mixed valid/invalid run `h4gumcKUWI1YRBABi` succeeded with dataset `d7efboksQTr8Mw5M8`: 1 Berlin row, 1 confirmed charge, and the invalid location skipped without charge.
- The default key-value store contained only `INPUT`; no per-item `OUTPUT` records were written.
- The new Actor had five recent successful runs at release verification. Portfolio secret and pricing audits reported 92/92 actors covered; the only health-audit failure was unrelated `tiktok-challenge-posts-scraper` (`CVaJEgPjl3jWKbm71`).

## Handoff result

Release gates passed. No code changes were required during merge/deploy. Existing unrelated workspace changes were preserved and not included in this handoff.

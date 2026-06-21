# Smoke Run Reports

Chronological evidence reports for Apify actor smoke runs. Run smoke checks before listing promotions, after Scrappa API changes, and during scheduled reliability sweeps; append new reports to the table in date order.

| Date | Report | Scope |
| --- | --- | --- |
| 2026-07-01 | [TrustedShops Reviews paid activation gate](./2026-07-01-trustedshops-reviews-paid-activation.md) | Scheduled post-activation verification for public actor `trustedshops-reviews-scraper` (`L5tTNPlxeCTlFUUjl`) after its `PAY_PER_EVENT` `review-result` pricing starts on 2026-07-01. |
| 2026-06-20 | [Booking Search stability gate](./2026-06-20-booking-search-stability-gate.md) | Three fresh `booking-search-scraper` (`BehWN3LEvBxhEiJDF`) runs passed with clean logs and dataset output before clearing `UNDER_MAINTENANCE`. |
| 2026-06-18 | [Google Trends Autocomplete paid activation pre-check](./2026-06-18-google-trends-autocomplete-pre-activation.md) | Public actor `google-trends-autocomplete-scraper` remains future-only paid before the scheduled Apify activation at `2026-06-19T09:30:30.582Z`; post-activation smoke run still required. |
| 2026-06-10 | [Social and video no-run actor smoke sweep](./2026-06-10-social-video-no-run-actors.md) | Nine public paid Instagram, TikTok, Trustpilot, and YouTube actors moved from owner-filtered `NO_RUNS` audit status to successful run evidence. |
| 2026-05-31 | [Google Finance no-run actor smoke sweep](./2026-05-31-google-finance-no-run-actors.md) | Four public paid Google Finance actors moved out of target `NO_RUNS`; historical prices has a tracked custom date-range follow-up after a successful preset-range rerun. |
| 2026-05-30 | [Google Maps no-run actor smoke sweep](./2026-05-30-google-maps-no-run-actors.md) | Six public paid Google Maps actors moved from owner-filtered `NO_RUNS` audit status to successful run evidence. |
| 2026-05-30 | [Stale maintenance notice clearance](./2026-05-30-stale-maintenance-notices.md) | Four public monetized TheScrappa actors verified before clearing stale `UNDER_MAINTENANCE` notices. |
| 2026-05-22 | [Public actors with no run-history sweep](./2026-05-22-public-actors-no-history.md) | Six public TheScrappa actors checked before further listing promotion. |

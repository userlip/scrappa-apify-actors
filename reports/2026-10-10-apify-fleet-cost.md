# Apify fleet cost optimization (2026-10-10)

Follow-up to the Similarweb negative-margin alert. PRs #411, #412, #413.

## Measured cost drivers (before)

- Single-item run (similarweb, 128 MB): $0.000154 = key-value writes $0.000100 (INPUT by Apify + OUTPUT by the Actor), compute $0.000043 (3.1 s, of which ~2 s idle settle wait before the first dataset write), dataset/reads ~$0.00001.
- Builds: Rust compiled on Apify's 4 GB builder, ~137 s / 0.152 CU / ~$0.06 per build. 2026-10-01: 111 builds $8.14; 2026-10-02: 791 builds $55.84.
- Users could start runs with more memory than the default (likely cause of the Similarweb loss).

## Changes

- Prebuilt binaries (`scripts/prebuilt-actor.mjs`): compiled locally in `rust:1.90-slim-bookworm`, uploaded gzipped, runtime-only Dockerfile. 2026-10-10 redeploy: 215 builds for $0.294 ($0.0014 per build, -98%).
- `min/maxMemoryMbytes` in every `actor.json` (177×128/128, 20×128/256, a few 256/256).
- First dataset offset lookup per run no longer sleeps 2 s. Single-item run 3.1 s → 1.9 s.
- Similarweb: 5 concurrent Scrappa fetches (30 domains 23.5 s → 7.5 s).
- 9 YouTube actors called ytapi.scrappa.co directly without credentials and fail since ytapi requires an internal key (2026-10-09). Moved to `https://scrappa.co/api/youtube/*` with `X-API-Key` (#413).
- scrappa-worker-1 lacked `YTAPI_INTERNAL_KEY` (about 1 in 4 public `/api/youtube/*` requests returned 401). Copied from scrappa-worker-2, config cached; 0/40 401 afterwards. Backup: `/home/ploi/.env.scrappa.bak-20261010` on worker-1.

## Live state

- 185/187 Actors on optimized builds. `instagram-trending-reels-scraper` and `mobile-de-listing-scraper` failed the candidate run on Scrappa HTTP 503 and are queued in `~/scrappa-apify-publisher/pending-deploy.txt` (retried every 4 h).
- Prefill returns 0 results (upstream data, same on old builds): booking-search, booking-reviews, youtube-api-playlists, -channel-podcasts, -get-channel-livestreams, -get-channel-shorts. Deployed with `--allow-empty`.

## Decisions applied (2026-10-10, approved)

- `apify-actor-start` event at $0.00005 (one-time, flat) added to the 102 Actors without it. Apify requires 14 days notice: effective 2026-10-24T15:17:56Z. All 187 Actors now have it in their latest pricing entry. From then on Apify pays the first 5 s of compute per run.
- `OUTPUT` record kept (documented for users).
- Shorter Scrappa retries (#414): 2 retries (1 s, 2 s backoff), 60 s entry budget; the first attempt keeps its 45 s timeout. Failed runs drop from 50-100 s to a few seconds when Scrappa fails fast. Apify API retries unchanged.
- Redeployed: 183/187 on the latest code. `google-finance-markets-scraper`, `google-maps-photos-scraper`, `instagram-trending-reels-scraper`, `mobile-de-listing-scraper` failed the candidate run on Scrappa HTTP 503 and are in `~/scrappa-apify-publisher/pending-deploy.txt` (retried every 4 h).

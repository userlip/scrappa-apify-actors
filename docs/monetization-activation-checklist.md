# Monetization Activation Checklist

Last live metadata check: 2026-05-11 via Apify Actor detail API for the `TheScrappa` organization.

This checklist tracks public Scrappa actors whose paid pricing is scheduled for future activation in May 2026. On each activation date, verify pricing from the Apify API or Console before treating the actor as monetized. Listing copy such as "$0.20/1k results" is not evidence of active paid pricing.

## Verification Standard

For every actor listed for a given date:

- [ ] Fetch the actor detail metadata from `GET /v2/acts/{actorId}` using the organization token.
- [ ] Confirm `isPublic` is still `true`.
- [ ] Confirm the expected `pricingInfos[].startedAt` timestamp is at or before the verification time.
- [ ] Confirm the active paid pricing is visible in Apify Console or API (`currentPricingInfo`, `pricingInfo`, or the current effective pricing panel).
- [ ] Confirm the actor is not effectively free: no missing pricing, no disabled pricing, no free-only pricing, and no future-only paid pricing after the activation timestamp has passed.
- [ ] Save the evidence in an audit note with actor ID, actor slug, verification timestamp, pricing model, event or unit price, and API/Console result.

If any check fails, treat it as P0: update or re-schedule paid pricing immediately at the earliest Apify-allowed date, record the blocker message, and do not mark that actor's activation complete.

Use this API command shape for evidence collection without committing secrets:

```bash
curl -s "https://api.apify.com/v2/acts/ACTOR_ID?token=$APIFY_TOKEN" \
  | jq '{id: .data.id, name: .data.name, isPublic: .data.isPublic, pricingInfo: .data.pricingInfo, currentPricingInfo: .data.currentPricingInfo, pricingInfos: .data.pricingInfos}'
```

## Exact-Date Activation Queue

### 2026-05-17

- [ ] `4DwyH8vZcinXWrHBA` - `google-jobs-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:39:48.217Z`.
- [ ] `HYG9AqNEDSHMHgH4O` - `google-news-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:43:29.941Z`.
- [ ] `mp03zGSA2pR31azfU` - `instagram-user-posts-cheapest-0-20-1000-results` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:43:29.941Z`.
- [ ] `8ejIZ0nfRPShvWBSP` - `scrappa-google-search` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:43:29.941Z`.
- [ ] `oaJANlheGg9o3EZjU` - `tiktok-comments-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:43:29.941Z`.
- [ ] `6ZUj6u4SWuJxOQnn9` - `youtube-api-batch-videos` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:43:29.941Z`.
- [ ] `w464EbPGGZqcmrC8j` - `youtube-api-channel-videos` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:44:29.173Z`.
- [ ] `S9Gf6PSqzz6hxvMNA` - `youtube-api-get-channel-community` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:43:29.941Z`.
- [ ] `Xhwtx7clQKnPRez1H` - `youtube-api-get-playlists-details` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:43:29.941Z`.
- [ ] `P1Jv1QuMoId4XUPlC` - `youtube-api-get-video-details` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:44:29.173Z`.
- [ ] `O1ltDU9qk4adR2x86` - `youtube-api-search-by-category` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:43:29.941Z`.
- [ ] `ziD2fUoLsdzKlc6zR` - `youtube-api-search-data` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:43:29.941Z`.
- [ ] `oecJ81oeff1KozCtd` - `youtube-api-search-suggestions` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:44:29.173Z`.
- [ ] `ZT2Z352FLhgqgtMrg` - `youtube-api-video-comments` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:44:29.173Z`.

### 2026-05-19

- [ ] `ElkkSkWZ7xAaOqsr4` - `tiktok-profile-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-19T10:40:00.000Z`.
- [ ] `iSnxQQAvqnI0ZKL9F` - `tiktok-user-posts-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-19T10:40:00.000Z`.
- [ ] `ztc698cHC09lkCDYE` - `youtube-transcript-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-19T11:18:38.521Z`.

### 2026-05-21

- [ ] `1WE6uJzTx1DbS5u39` - `tiktok-followers-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-21T08:15:00.000Z`.
- [ ] `a3CzWl85xlYKi9UIn` - `tiktok-following-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-21T08:30:00.000Z`.

### 2026-05-22

- [ ] `GAAKVpkPvj3lMbO6G` - `linkedin-jobs-search-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-22T10:35:00.000Z`.

### 2026-05-23

- [ ] `aE7VcbT6CIWBxob7U` - `google-finance-quote-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-23T12:40:05.000Z`.
- [ ] `nBSazp2iBmBm1FQvz` - `trustpilot-company-reviews-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-23T22:55:39.839Z`.

### 2026-05-24

- [ ] `H2dZTreGZ7s3XJsQ7` - `tiktok-hashtag-posts-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-24T00:00:00.000Z`.
- [ ] `MrbqFgdpNTQcRW0Vt` - `google-images-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-24T07:16:00.000Z`.

## Portfolio Backstop

Run this backstop on every activation date after checking the due actors:

- [ ] List all `TheScrappa` actors through `GET /v2/acts?my=1`.
- [ ] For every actor where `isPublic` is `true`, fetch `GET /v2/acts/{actorId}`.
- [ ] Flag any public actor with `pricingInfo: null`, `pricingInfos: null`, an empty `pricingInfos` array, or no pricing entry whose `startedAt` is at or before the verification time.
- [ ] Flag any public actor whose only paid pricing starts in the future.
- [ ] Record all flagged actor IDs, slugs, blocker messages, and the exact follow-up action/date.

The activation audit is complete only when every public actor is either actively paid or has a documented Apify blocker with the earliest allowed paid activation date.

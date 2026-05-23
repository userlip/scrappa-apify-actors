# Monetization Activation Checklist

Last live metadata check: 2026-05-23T11:15Z via Apify Actor list/detail API for the `TheScrappa` organization, `scripts/audit-apify-pricing.mjs --json --include-active`, `scripts/audit-apify-secrets.mjs --json --include-present`, and local actor manifest count.

Current inventory backstop for every activation audit:

- 72 live `thescrappa` actors in Apify.
- 62 public `thescrappa` actors in Apify according to the pricing and secret audits.
- 64 local actor manifests in this repo; all 64 are represented by live Apify actors.
- 8 live actors still missing local source directories here.
- Pricing audit: 0 public actors missing paid pricing, 0 overdue active-pricing gaps, 46 public actors with active paid evidence, and 16 public actors with future-only paid pricing.
- Secret audit: all 62 public actors have `SCRAPPA_API_KEY` configured as a secret.
- Run-health notes: `stepstone-jobs-scraper` (`DUUlFa5LGId75vOI0`) latest run `wr3oYXhiE25DmrsUH` failed with Scrappa API `422` validation for `work_from_home`; `youtube-transcript-scraper` (`ztc698cHC09lkCDYE`) still has Apify notice `UNDER_MAINTENANCE` despite latest run `hRCogd7KpVQU9oRJ5` succeeding; `google-hotels-search-scraper` (`Kc3rfsV2Hif23mctw`) still has Apify notice `UNDER_MAINTENANCE` despite latest run `bfhYjZNMzgn8wEaR4` succeeding; `booking-search-scraper` (`BehWN3LEvBxhEiJDF`) latest run `fGubXwuYYNmoMyakt` succeeded.

This checklist tracks Scrappa actors that were public on 2026-05-11 and whose paid pricing is scheduled for future activation, amended with newly published actors that now have scheduled paid pricing. On each activation date, verify pricing from the Apify API or Console before treating the actor as monetized.
Listing copy such as "$0.20/1k results" is not evidence of active paid pricing.

Actors can be included here even when their source files are missing locally; the audit scope is the live Apify actor, not local repository coverage.
Actors that were private on 2026-05-11 are excluded from the exact-date queue until they become public, but the portfolio backstop below must catch them before or when publication happens.

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
curl -s -H "Authorization: Bearer $APIFY_TOKEN" \
  "https://api.apify.com/v2/acts/ACTOR_ID" \
  | jq '{id: .data.id, name: .data.name, isPublic: .data.isPublic, pricingInfo: .data.pricingInfo, currentPricingInfo: .data.currentPricingInfo, pricingInfos: .data.pricingInfos}'
```

## Exact-Date Activation Queue

### 2026-05-17

Past-due items in this section should be rechecked against the latest pricing audit before changing checkbox state; some may already have active paid evidence in the README inventory table.

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
- [ ] `eKzA6GhEOJACIiCUW` - `youtube-api-related-videos` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:44:29.173Z` with `apify-default-dataset-item` priced at `$0.0002-$0.0003/result` by tier.
- [ ] `O1ltDU9qk4adR2x86` - `youtube-api-search-by-category` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:43:29.941Z`.
- [ ] `ziD2fUoLsdzKlc6zR` - `youtube-api-search-data` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:43:29.941Z`.
- [ ] `oecJ81oeff1KozCtd` - `youtube-api-search-suggestions` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:44:29.173Z`.
- [ ] `T7ddx0tgVCwMHi9ET` - `youtube-api-trending-videos` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:44:29.173Z`.
- [ ] `ZT2Z352FLhgqgtMrg` - `youtube-api-video-comments` - verify `PAY_PER_EVENT` activation at `2026-05-17T14:44:29.173Z`.

### 2026-05-19

- [ ] `ElkkSkWZ7xAaOqsr4` - `tiktok-profile-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-19T10:40:00.000Z`.
- [ ] `iSnxQQAvqnI0ZKL9F` - `tiktok-user-posts-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-19T10:40:00.000Z`.
- [ ] `ztc698cHC09lkCDYE` - `youtube-transcript-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-19T11:18:38.521Z`.

### 2026-05-21

- [ ] `1WE6uJzTx1DbS5u39` - `tiktok-followers-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-21T08:15:00.000Z`.
- [ ] `a3CzWl85xlYKi9UIn` - `tiktok-following-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-21T08:30:00.000Z`.
- [ ] `4DSOKG4JhcS4lhu60` - `tiktok-video-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-21T08:59:06.000Z` with `apify-default-dataset-item` priced at `$0.0002/result`.

### 2026-05-22

- [ ] `GAAKVpkPvj3lMbO6G` - `linkedin-jobs-search-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-22T10:35:00.000Z`.

### 2026-05-23

- [ ] `aE7VcbT6CIWBxob7U` - `google-finance-quote-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-23T12:40:05.000Z`.
- [ ] `nBSazp2iBmBm1FQvz` - `trustpilot-company-reviews-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-23T22:55:39.839Z`.

### 2026-05-24

- [ ] `H2dZTreGZ7s3XJsQ7` - `tiktok-hashtag-posts-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-24T00:00:00.000Z`.
- [ ] `MrbqFgdpNTQcRW0Vt` - `google-images-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-24T07:16:00.000Z`.

### 2026-05-29

- [ ] `DUUlFa5LGId75vOI0` - `stepstone-jobs-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-29T11:31:58.000Z` with `apify-default-dataset-item` priced at `$0.0003/result`.
- [ ] `WvbWRqj67ve6fwwWZ` - `google-finance-markets-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-29T08:13:17.000Z`.

### 2026-05-31

- [ ] `EiUCYz2MjYUuGT6Xu` - `arbeitsagentur-jobs-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-31T08:10:00.000Z` with `apify-default-dataset-item` priced at `$0.0003/result`.
- [ ] `kAdTwn5fkBCGKOQUq` - `google-videos-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-31T15:11:49.000Z` with `apify-default-dataset-item` priced at `$0.0002-$0.0003/result` by tier.

### 2026-06-01

- [ ] `u8F5YhfXkQIrgLe73` - `vinted-search-scraper` - verify `PAY_PER_EVENT` activation at `2026-06-01T07:00:00.000Z` with `item-result` priced at `$0.0002/result`.

### 2026-06-02

- [ ] `W8yULHo0Mzq7CYRrM` - `trustpilot-business-search-scraper` - verify `PAY_PER_EVENT` activation at `2026-06-02T08:19:17.249Z` with `business-result` priced at `$0.0002/result`.
- [ ] `8SvzPgdsdg1yZK1t4` - `jameda-search-scraper` - verify `PAY_PER_EVENT` activation at `2026-06-02T12:51:18.290Z` with `doctor-result` priced at `$0.0002/result`.

### 2026-06-03

- [ ] `BehWN3LEvBxhEiJDF` - `booking-search-scraper` - verify `PAY_PER_EVENT` activation at `2026-06-03T10:06:39.118Z` with `booking-result` priced at `$0.0002/result`; also confirm the dated input-schema prefills and README examples are still future dates before marketplace promotion.

### 2026-06-04

- [ ] `MDgsOkRoh1bAfC28g` - `similarweb-traffic-analytics-scraper` - verify `PAY_PER_EVENT` activation at `2026-06-04T11:52:30.000Z` with `domain-result` priced at `$0.0002/result`.

### 2026-06-05

- [ ] `601ilBYtO52NNsMrT` - `immobilienscout24-search-scraper` - verify `PAY_PER_EVENT` activation at `2026-06-05T06:59:39.484Z` with `property-result` priced at `$0.0003/result`.
- [ ] `Xa9ClmgD4tI9lHT91` - `redfin-property-search-scraper` - verify `PAY_PER_EVENT` activation at `2026-06-05T07:59:30.000Z` with `property-result` priced at `$0.0003/result`.

## Portfolio Backstop

Run this backstop on every activation date after checking the due actors:

- [ ] List all `TheScrappa` actors through `GET /v2/acts?my=1`.
- [ ] Confirm the live inventory count against the README before starting the audit; the 2026-05-23T11:15Z baseline on this branch is 72 live actors, 62 public actors, 64 local actor manifests, and 8 missing local source directories.
- [ ] For every actor where `isPublic` is `true`, fetch `GET /v2/acts/{actorId}`.
- [ ] Flag any public actor with `pricingInfo: null`, `pricingInfos: null`, an empty `pricingInfos` array, or no pricing entry whose `startedAt` is at or before the verification time.
- [ ] Flag any public actor whose only paid pricing starts in the future.
- [ ] Record all flagged actor IDs, slugs, blocker messages, and the exact follow-up action/date.

The activation audit is complete only when every public actor is either actively paid or has a documented Apify blocker with the earliest allowed paid activation date.

## Private Scheduled Actors

These actors also had May 17, 2026 scheduled pricing in the live API, but `isPublic` was `false` on 2026-05-11.
They are intentionally excluded from the public activation queue above and must be added to the queue before publication if they are made public:

- `Y3mKYlGNhsrBE7aZO` - `youtube-api-channel-podcasts`
- `vKqlzEXa47Ubpuix5` - `youtube-api-get-channel-about-details`
- `svtEvWGEssObsU72e` - `youtube-api-get-channel-details`
- `WT3XhaJ0lUYmp0eFu` - `youtube-api-get-channel-livestreams`
- `3ERhmU2MUBjdR4AOq` - `youtube-api-get-channel-playlists`
- `608amsD2lD6xRKbax` - `youtube-api-get-channel-shorts`
- `yJiDippxXaK5hWQRC` - `youtube-api-get-channel-statistics`
- `N5ol78TtqiMj4MtM6` - `youtube-api-hashtags`
- `hueJrwkrbo20Ufrna` - `youtube-api-playlists`
- `fq5Kq9OfBRWYu9go1` - `youtube-api-video-chapters`

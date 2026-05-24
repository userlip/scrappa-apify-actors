# Monetization Activation Checklist

Last live metadata check: 2026-05-24T07:50:47.636Z via Apify Actor list/detail API for the `TheScrappa` organization, direct `GET /v2/acts/{actorId}` for the May 24 actors, recent runs/builds API, `scripts/audit-apify-pricing.mjs --json --include-active`, `scripts/audit-apify-secrets.mjs --json --include-present`, and local actor manifest count.

Current inventory backstop for every activation audit:

- 74 live `thescrappa` actors in Apify.
- 64 public `thescrappa` actors in Apify according to the pricing and secret audits.
- 65 local actor manifests in this repo; all 65 are represented by live Apify actors.
- 9 live actors still missing local source directories here; `google-search-scraper` is represented by the legacy `actors/google-search` directory.
- Pricing audit at 2026-05-24T07:50:40.247Z: 0 public actors missing paid pricing, 0 overdue active-pricing gaps, 51 public actors with active paid evidence, and 13 public actors with future-only paid pricing.
- Secret audit at 2026-05-24T07:50:47.636Z: all 64 public actors have `SCRAPPA_API_KEY` configured as a secret.
- May 24 activation evidence at 2026-05-24T07:23:43.706Z: `tiktok-hashtag-posts-scraper` (`H2dZTreGZ7s3XJsQ7`) and `google-images-scraper` (`MrbqFgdpNTQcRW0Vt`) are public with active `PAY_PER_EVENT` evidence from `pricingInfos`; Apify still reports `pricingInfo/currentPricingInfo: null` for both.
- Run-health notes: no latest run failures were returned by the all-actor run sweep; `youtube-transcript-scraper` (`ztc698cHC09lkCDYE`) and `google-hotels-search-scraper` (`Kc3rfsV2Hif23mctw`) still have Apify notice `UNDER_MAINTENANCE` while their monetization remains configured.

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

- [x] `4DwyH8vZcinXWrHBA` - `google-jobs-scraper` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `HYG9AqNEDSHMHgH4O` - `google-news-scraper` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `mp03zGSA2pR31azfU` - `instagram-user-posts-cheapest-0-20-1000-results` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `8ejIZ0nfRPShvWBSP` - `scrappa-google-search` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `oaJANlheGg9o3EZjU` - `tiktok-comments-scraper` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `6ZUj6u4SWuJxOQnn9` - `youtube-api-batch-videos` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `w464EbPGGZqcmrC8j` - `youtube-api-channel-videos` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `S9Gf6PSqzz6hxvMNA` - `youtube-api-get-channel-community` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `Xhwtx7clQKnPRez1H` - `youtube-api-get-playlists-details` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `P1Jv1QuMoId4XUPlC` - `youtube-api-get-video-details` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `eKzA6GhEOJACIiCUW` - `youtube-api-related-videos` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24 with `apify-default-dataset-item` priced at `$0.0002-$0.0003/result` by tier.
- [x] `O1ltDU9qk4adR2x86` - `youtube-api-search-by-category` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `ziD2fUoLsdzKlc6zR` - `youtube-api-search-data` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `oecJ81oeff1KozCtd` - `youtube-api-search-suggestions` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `T7ddx0tgVCwMHi9ET` - `youtube-api-trending-videos` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `ZT2Z352FLhgqgtMrg` - `youtube-api-video-comments` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.

### 2026-05-19

- [x] `ElkkSkWZ7xAaOqsr4` - `tiktok-profile-scraper` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `iSnxQQAvqnI0ZKL9F` - `tiktok-user-posts-scraper` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `ztc698cHC09lkCDYE` - `youtube-transcript-scraper` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.

### 2026-05-21

- [x] `1WE6uJzTx1DbS5u39` - `tiktok-followers-scraper` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `a3CzWl85xlYKi9UIn` - `tiktok-following-scraper` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `4DSOKG4JhcS4lhu60` - `tiktok-video-scraper` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24 with `apify-default-dataset-item` priced at `$0.0002/result`.

### 2026-05-22

- [x] `GAAKVpkPvj3lMbO6G` - `linkedin-jobs-search-scraper` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.

### 2026-05-23

- [x] `aE7VcbT6CIWBxob7U` - `google-finance-quote-scraper` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.
- [x] `nBSazp2iBmBm1FQvz` - `trustpilot-company-reviews-scraper` - verified active `PAY_PER_EVENT` pricing in `pricingInfos` on 2026-05-24.

### 2026-05-24

- [x] `H2dZTreGZ7s3XJsQ7` - `tiktok-hashtag-posts-scraper` - verified `PAY_PER_EVENT` activation at `2026-05-24T07:23:43.706Z`; direct actor detail returned `isPublic: true`, active evidence from `pricingInfos` started `2026-05-24T00:00:00.000Z`, `apify-default-dataset-item` at `$0.0003/result`, latest run `9jme9tLGVrekgx8RW` succeeded, latest build `adY4fybFQ8GgZ2iXQ` succeeded, and `SCRAPPA_API_KEY` is secret on version `1.0`.
- [x] `MrbqFgdpNTQcRW0Vt` - `google-images-scraper` - verified `PAY_PER_EVENT` activation at `2026-05-24T07:23:43.706Z`; direct actor detail returned `isPublic: true`, active evidence from `pricingInfos` started `2026-05-24T07:16:00.000Z`, `apify-default-dataset-item` tiered pricing `FREE $0.0003`, `BRONZE $0.00025`, `SILVER $0.00022`, and `GOLD/PLATINUM/DIAMOND $0.0002`, latest run `GN4vccA6MYYEe3KDO` succeeded, latest build `psT5nsFjo6WkUpw3q` succeeded, and `SCRAPPA_API_KEY` is secret on version `1.0`.

### 2026-05-26

- [ ] `Kc3rfsV2Hif23mctw` - `google-hotels-search-scraper` - verify `PAY_PER_EVENT` activation at `2026-05-26T08:23:20.330Z` with `apify-default-dataset-item` priced at `$0.0002/result`.

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

### 2026-06-07

- [ ] `hoF0Qgm3S0wAfpY8y` - `google-trends-related-queries-scraper` - verify `PAY_PER_EVENT` activation at `2026-06-07T08:00:00.000Z` with `related-result` priced at `$0.0002/result`; Apify blocked immediate pricing on 2026-05-24 with `cannot-modify-actor-pricing-with-immediate-effect`.

## Portfolio Backstop

Run this backstop on every activation date after checking the due actors:

- [ ] List all `TheScrappa` actors through `GET /v2/acts?my=1`.
- [ ] Confirm the live inventory count against the README before starting the audit; the 2026-05-24T07:50:47.636Z baseline on this branch is 74 live actors, 64 public actors, 65 local actor manifests, and 9 missing local source directories.
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
- `c1Lt3EdNOuoen5In7` - `tiktok-ads-scraper` - private on 2026-05-24 with `PAY_PER_EVENT` pricing active from `2026-05-24T07:22:00.000Z`; add to the public queue before publication.

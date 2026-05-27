# Scrappa Apify Actors

Apify Actors for [Scrappa](https://scrappa.co) APIs - Google Search, Maps, YouTube, LinkedIn, and more.

## Structure

```
scrappa-apify-actors/
├── actors/           # Individual Apify actors
│   ├── google-search/
│   ├── google-maps-business-details-scraper/
│   └── ...
├── shared/           # Shared Scrappa client library
└── package.json      # Workspace root
```

## Setup

```bash
# Install pnpm if you don't have it
npm install -g pnpm

# Install dependencies
pnpm install

# Build all packages
pnpm build
```

## Development

### Running an actor locally

```bash
cd actors/google-search
pnpm start:dev
```

### Auditing scheduled pricing activation

Public actors must have active `pricingInfo` or `currentPricingInfo` once scheduled paid pricing takes effect. Run the live Apify pricing audit with an organization token:

```bash
APIFY_TOKEN=... pnpm audit:pricing
```

For an exact activation checkpoint, pass the verification timestamp:

```bash
APIFY_TOKEN=... pnpm audit:pricing --now 2026-05-17T15:00:00.000Z
```

The audit exits with code `1` when a public actor has due paid `pricingInfos` but no active paid evidence in `pricingInfo` or `currentPricingInfo`, or when a public actor has no paid `pricingInfos`.

### Auditing actor secrets

Public actors that call Scrappa APIs must have `SCRAPPA_API_KEY` configured as a secret on their default Actor version. The secret audit resolves the default version from Apify `defaultRunOptions.build` and the available version `buildTag` values before checking env vars, so legacy `0.0` Actors and newer `1.0` Actors are both checked correctly:

```bash
APIFY_TOKEN=... pnpm audit:secrets
```

For machine-readable evidence, use:

```bash
APIFY_TOKEN=... pnpm audit:secrets --json --include-present
```

### Verifying live actor source parity

Before deployment work, compare the live TheScrappa Apify inventory against local actor sources:

```bash
APIFY_TOKEN=... pnpm verify:live-actors
```

To import live `SOURCE_FILES` for actors that are missing locally without overwriting existing actor directories, run:

```bash
APIFY_TOKEN=... pnpm import:live-actors
```

### Creating a new actor

1. Copy an existing actor as a template:
```bash
cp -r actors/google-search actors/my-new-actor
```

2. Update `package.json` with the new name
3. Update `.actor/actor.json` with metadata
4. Update `.actor/input_schema.json` with input fields
5. Implement the actor logic in `src/main.ts`

## Deploying to Apify

### First time setup

```bash
# Login to Apify
apify login

# Push an actor
cd actors/google-search
apify push
```

### Updating an actor

```bash
cd actors/google-search
pnpm build
apify push
```

## Available Actors

This inventory is aligned to the live `TheScrappa` Apify org as of 2026-05-26T19:25:14.000Z. It lists all 75 live `thescrappa` actors. Actor versions use Apify `SOURCE_FILES`; the live metadata does not indicate a Git-linked Apify source.

Current coverage in this repository:
- 75 live `thescrappa` actors in Apify
- 65 public `thescrappa` actors in Apify according to the pricing and secret audits
- 75 local actor source directories in this repo; all 75 are represented by live Apify actors
- 0 live actors missing a local source directory here; `google-search-scraper` is represented by the legacy `actors/google-search` directory
- Pricing follow-up: `website-content-extractor-scraper` is public with paid `PAY_PER_EVENT` pricing scheduled for 2026-06-09T19:25:00.000Z after Apify blocked immediate pricing with a two-week lead-time rule.
- Secret follow-up: the new `website-content-extractor-scraper` version has `SCRAPPA_API_KEY` configured as a secret.
- Run-health notes from the current audit pass: no latest run failures were returned by the all-actor run sweep; `youtube-transcript-scraper` (`ztc698cHC09lkCDYE`) and `google-hotels-search-scraper` (`Kc3rfsV2Hif23mctw`) still have Apify notice `UNDER_MAINTENANCE` while their monetization remains configured.

Use [docs/monetization-activation-checklist.md](docs/monetization-activation-checklist.md) for the exact-date May 2026 activation audit of actors that were public on 2026-05-11.

| Local directory | Apify actor | Actor ID | Title | Source coverage | Pricing follow-up |
|---|---|---|---|---|---|
| `actors/arbeitsagentur-jobs-scraper` | `arbeitsagentur-jobs-scraper` | `EiUCYz2MjYUuGT6Xu` | Arbeitsagentur Jobs Scraper ($0.30/1k results) | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-31 (`PAY_PER_EVENT`, default dataset job result at $0.0003/result); verify activation |
| `actors/booking-search-scraper` | `booking-search-scraper` | `BehWN3LEvBxhEiJDF` | Booking.com Search Scraper | Local source present; live Apify version uses `SOURCE_FILES`; latest run `fGubXwuYYNmoMyakt` succeeded | Scheduled 2026-06-03 (`PAY_PER_EVENT`, `booking-result` at $0.0002/result); Apify pricing schedule created 2026-05-20 with two-week lead time |
| `actors/google-finance-quote-scraper` | `google-finance-quote-scraper` | `aE7VcbT6CIWBxob7U` | Google Finance Quote Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-23 (`PAY_PER_EVENT`); verify activation |
| `actors/google-finance-historical-prices-scraper` | `google-finance-historical-prices-scraper` | `zqckCpYtvPvxb2oxl` | Google Finance Historical Prices Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Public with `pricingInfos` active from 2026-05-13 (`PAY_PER_EVENT`, `price-point` at $0.0002/result); verify `currentPricingInfo` if Apify API begins populating it |
| `actors/google-finance-markets-scraper` | `google-finance-markets-scraper` | `WvbWRqj67ve6fwwWZ` | Google Finance Markets Scraper | Local source present; live Apify version uses `SOURCE_FILES`; cloud run `DzKsppuCjSGCv6vMb` succeeded | Scheduled 2026-05-29 (`PAY_PER_EVENT`, `market-item` at $0.0002/result); Apify blocks immediate pricing with two-week lead time |
| `actors/google-finance-search-scraper` | `google-finance-search-scraper` | `JiWVC7gsvVEfwCxKv` | Google Finance Search Scraper | Local source present; live Apify version uses `SOURCE_FILES`; build `pOIUDsrmlV97QWPwH` succeeded; smoke runs `z0f3hhoT7etMxhWXW`, `xvbgwTV4F6aeO9Qlk`, and post-public run `hjmdEGyUEfioy9THV` succeeded | Public with `pricingInfos` active from 2026-05-23T08:00:00Z (`PAY_PER_EVENT`, `finance-search-result` at $0.0002/result); API still reports `pricingInfo/currentPricingInfo: null` immediately after publication |
| `actors/google-flights-search-scraper` | `google-flights-search-scraper` | `IIPXRhbeyXH7ssOK6` | Google Flights Search Scraper | Local source present; live Apify version uses `SOURCE_FILES` | `pricingInfos` start 2026-05-11 (`PAY_PER_EVENT`, `flight-result` at $0.0002/result); cloud run verified |
| `actors/google-hotels-search-scraper` | `google-hotels-search-scraper` | `Kc3rfsV2Hif23mctw` | Google Hotels Search Scraper | Local source present; live Apify version uses `SOURCE_FILES`; Apify notice still `UNDER_MAINTENANCE`; latest run `bfhYjZNMzgn8wEaR4` succeeded | Scheduled 2026-05-26 (`PAY_PER_EVENT`, default dataset hotel result at $0.0002/result); verify activation |
| `actors/google-images-scraper` | `google-images-scraper` | `MrbqFgdpNTQcRW0Vt` | Google Images Scraper | Local source present; live Apify version uses `SOURCE_FILES`; latest run `GN4vccA6MYYEe3KDO` succeeded; latest build `psT5nsFjo6WkUpw3q` succeeded | Active `PAY_PER_EVENT` pricing verified 2026-05-24T07:23:43.706Z from `pricingInfos` (`apify-default-dataset-item` tiered pricing: FREE $0.0003, BRONZE $0.00025, SILVER $0.00022, GOLD/PLATINUM/DIAMOND $0.0002/result); `pricingInfo/currentPricingInfo` still null |
| `actors/google-jobs-scraper` | `google-jobs-scraper` | `4DwyH8vZcinXWrHBA` | Google Jobs Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/google-videos-scraper` | `google-videos-scraper` | `kAdTwn5fkBCGKOQUq` | Google Videos Scraper | Local source present; live Apify version uses `SOURCE_FILES`; cloud run `Ca8I8MvC2mh6a5bxo` succeeded with 11 dataset items | Scheduled 2026-05-31 (`PAY_PER_EVENT`, default dataset video result at $0.0002-$0.0003/result by tier); Apify blocked immediate pricing with two-week lead time |
| `actors/google-maps-advanced-search-scraper` | `google-maps-advanced-search-scraper` | `DT8bUdm2Vn4HjlyDo` | Google Maps Advanced Search Scraper | Local source present; live Apify version uses `SOURCE_FILES` | `pricingInfos` start 2025-12-21 (`PRICE_PER_DATASET_ITEM`); verify `currentPricingInfo` |
| `actors/google-maps-autocomplete-scraper` | `google-maps-autocomplete-scraper` | `hhS8GkceJHFiexWe6` | Google Maps Autocomplete Scraper | Local source present; live Apify version uses `SOURCE_FILES` | `pricingInfos` start 2025-12-22 (`PAY_PER_EVENT`); verify `currentPricingInfo` |
| `actors/google-maps-business-details-scraper` | `google-maps-business-details-scraper` | `JCqaAyY3Vy7K5UoRd` | Google Maps Business Details Scraper | Local source present; live Apify version uses `SOURCE_FILES` | `pricingInfos` start 2025-12-22 (`PAY_PER_EVENT`); verify `currentPricingInfo` |
| `actors/google-maps-photos-scraper` | `google-maps-photos-scraper` | `gLbfii9Nq4H7auMnN` | Google Maps Photos Scraper | Local source present; live Apify version uses `SOURCE_FILES` | `pricingInfos` start 2025-12-22 (`PAY_PER_EVENT`); verify `currentPricingInfo` |
| `actors/google-maps-reviews-scraper` | `google-maps-reviews-scraper` | `QvxzSeJiQrMggt1Vn` | Google Maps Reviews Scraper | Local source present; live Apify version uses `SOURCE_FILES` | `pricingInfos` start 2025-12-21 (`PRICE_PER_DATASET_ITEM`); verify `currentPricingInfo` |
| `actors/google-maps-search-scraper` | `google-maps-search-scraper` | `3fXhf8bJruXVWgDKy` | Google Maps Search Scraper | Local source present; live Apify version uses `SOURCE_FILES` | `pricingInfos` start 2025-12-21 (`PRICE_PER_DATASET_ITEM`); verify `currentPricingInfo` |
| `actors/google-news-scraper` | `google-news-scraper` | `HYG9AqNEDSHMHgH4O` | Google News Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/google-patents-search-scraper` | `google-patents-search-scraper` | `bSdKk0P65oTGDXLIh` | Google Patents Search Scraper | Local source present; live Apify version uses `SOURCE_FILES`; latest run `ol4X3z8GNKN6HdsV2` succeeded | `pricingInfos` active from 2026-05-14 (`PAY_PER_EVENT`, default dataset patent result at $0.0002/result); verify `currentPricingInfo` if Apify API begins populating it |
| `actors/google-search` | `google-search-scraper` | `2pU7EbKhShUz8BAnN` | Google Search Scraper | Local source present under legacy directory name; live Apify version uses `SOURCE_FILES` | `pricingInfos` start 2026-01-03 (`PRICE_PER_DATASET_ITEM`); verify `currentPricingInfo` |
| `actors/google-trends-interest-scraper` | `google-trends-interest-scraper` | `1D1neAFKb8LnbKvHG` | Google Trends Interest Scraper | Local source present; live Apify version uses `SOURCE_FILES` | `pricingInfos` start 2026-05-09 (`PAY_PER_EVENT`); verify `currentPricingInfo` |
| `actors/google-trends-related-queries-scraper` | `google-trends-related-queries-scraper` | `hoF0Qgm3S0wAfpY8y` | Google Trends Related Queries Scraper | Local source present; live build `RRFdmgbycbLLPix9o` / `1.0.1` succeeded; smoke run `LOioWx3QPzASdwian` succeeded with 25 dataset rows for `coffee` in `US` | Public with `PAY_PER_EVENT` pricing scheduled for 2026-06-07T08:00:00.000Z, `related-result` at $0.0002/result; Apify blocked immediate pricing with two-week lead time |
| `actors/indeed-jobs-scraper` | `indeed-jobs-scraper` | `OVlDREBAcO4iPyW64` | Indeed Jobs Scraper | Local source present; live Apify version uses `SOURCE_FILES` | `pricingInfos` start 2026-05-10 (`PAY_PER_EVENT`); verify `currentPricingInfo` |
| `actors/immobilienscout24-search-scraper` | `immobilienscout24-search-scraper` | `601ilBYtO52NNsMrT` | ImmobilienScout24 Search Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-06-05 (`PAY_PER_EVENT`, `property-result` at $0.0003/result); Apify pricing schedule created with two-week lead time |
| `actors/immowelt-property-search-scraper` | `immowelt-property-search-scraper` | `RtBugF27PKeDYceRA` | Immowelt Property Search Scraper | Local source present; live Apify version uses `SOURCE_FILES` | `pricingInfos` active from 2026-05-16 (`PAY_PER_EVENT`, `property-result` at $0.0003/result); verify `currentPricingInfo` if Apify API begins populating it |
| `actors/jameda-search-scraper` | `jameda-search-scraper` | `8SvzPgdsdg1yZK1t4` | Jameda Search Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-06-02 (`PAY_PER_EVENT`, `doctor-result` at $0.0002/result); verify activation |
| `actors/kleinanzeigen-search-scraper` | `kleinanzeigen-search-scraper` | `OTzcE9BHg83MpDprO` | Kleinanzeigen Search Scraper | Local source present; live Apify version uses `SOURCE_FILES`; cloud run `UphOLJdyQU5jTx47Q` succeeded; build `4oPbUr5dbd0tcpD1l` succeeded | Public with `pricingInfos` active from 2026-05-22 (`PAY_PER_EVENT`, `listing-result` at $0.00025/result) |
| `actors/stepstone-jobs-scraper` | `stepstone-jobs-scraper` | `DUUlFa5LGId75vOI0` | Stepstone Jobs Scraper ($0.30/1k results) | Local source present; live Apify version uses `SOURCE_FILES`; latest run `D9kHnhyxxBsHeFf8G` succeeded | Scheduled 2026-05-29 (`PAY_PER_EVENT`, default dataset job result at $0.0003/result); Apify pricing schedule created 2026-05-15 with two-week lead time |
| `actors/instagram-post-info-cheapest-0-20-1000-results` | `instagram-post-info-cheapest-0-20-1000-results` | `nfdzs1z0cRIU1Bfhw` | Instagram Post Info &#124; Cheapest $0.20/1k results | Local source present; live Apify version uses `SOURCE_FILES` | `pricingInfos` start 2025-02-01 (`PRICE_PER_DATASET_ITEM`); verify `currentPricingInfo` |
| `actors/instagram-user-info-cheapest-0-20-1000-results` | `instagram-user-info-cheapest-0-20-1000-results` | `VZrsJ6bO3h92y0duj` | Instagram User Info &#124; Cheapest $0.20/1k results | Local source present; live Apify version uses `SOURCE_FILES` | `pricingInfos` start 2025-01-31 (`PRICE_PER_DATASET_ITEM`); verify `currentPricingInfo` |
| `actors/instagram-user-posts-cheapest-0-20-1000-results` | `instagram-user-posts-cheapest-0-20-1000-results` | `mp03zGSA2pR31azfU` | Instagram User Posts Cheapest 0 20 1000 Results | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/linkedin-company-scraper` | `linkedin-company-scraper` | `EMGCTVXuOBRERiDMf` | LinkedIn Company Scraper - $0.30/1k results | Local source present; live Apify version uses `SOURCE_FILES` | `pricingInfos` start 2025-12-20 (`PRICE_PER_DATASET_ITEM`); verify `currentPricingInfo` |
| `actors/linkedin-jobs-search-scraper` | `linkedin-jobs-search-scraper` | `GAAKVpkPvj3lMbO6G` | LinkedIn Jobs Search Scraper ($0.30/1k results) | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-22 (`PAY_PER_EVENT`); verify activation |
| `actors/linkedin-post-scraper` | `linkedin-post-scraper` | `hVDOXgRoKJbnATxzs` | LinkedIn Post Scraper - $0.30/1k results | Local source present; live Apify version uses `SOURCE_FILES` | `pricingInfos` start 2025-12-20 (`PRICE_PER_DATASET_ITEM`); verify `currentPricingInfo` |
| `actors/linkedin-profile-scraper` | `linkedin-profile-scraper` | `87AaxKjjQrK0F0g60` | LinkedIn Profile Scraper - $0.30/1k results | Local source present; live Apify version uses `SOURCE_FILES` | `pricingInfos` start 2025-12-20 (`PRICE_PER_DATASET_ITEM`); verify `currentPricingInfo` |
| `actors/pinterest-search-scraper` | `pinterest-search-scraper` | `LXBYlqViHcR0tnt5i` | Pinterest Search Scraper | Local source present; live Apify version uses `SOURCE_FILES`; build `LMfYWckarkOkzTe0d` succeeded; public smoke run `Z6Jn8k0enEuldZMzT` succeeded with 1 dataset item for `home decor`, dataset `Lssox0TKprm4VeqAe`, `OUTPUT.pins_extracted = 1`, `OUTPUT.pins_saved = 1`, `OUTPUT.responses[0].pins.length = 1`, and `OUTPUT.query_summaries[0].fetch_limit = 3`; earlier paid smoke run `snXnWc1SsMyIcy8al` charged `pin-result = 3` | Public with `pricingInfos` active from 2026-05-23T11:13:51Z (`PAY_PER_EVENT`, `pin-result` at $0.0002/result); latest smoke run returned run-level `pricingInfo` with the same paid event |
| `actors/redfin-property-search-scraper` | `redfin-property-search-scraper` | `Xa9ClmgD4tI9lHT91` | Redfin Property Search Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-06-05 (`PAY_PER_EVENT`, `property-result` at $0.0003/result); Apify pricing schedule created with two-week lead time |
| `actors/scrappa-google-search` | `scrappa-google-search` | `8ejIZ0nfRPShvWBSP` | Scrappa Google Search | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/similarweb-traffic-analytics-scraper` | `similarweb-traffic-analytics-scraper` | `MDgsOkRoh1bAfC28g` | Similarweb Traffic Analytics Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-06-04 (`PAY_PER_EVENT`, `domain-result` at $0.0002/analyzed domain); Apify pricing schedule created 2026-05-21 with two-week lead time |
| `actors/tiktok-ads-scraper` | `tiktok-ads-scraper` | `c1Lt3EdNOuoen5In7` | TikTok Ads Scraper | Local source imported from live Apify `SOURCE_FILES` on 2026-05-24 | Private actor with `pricingInfos` active from 2026-05-24T07:22:00Z (`PAY_PER_EVENT`, default dataset item at $0.0002/result); add to public queue before publication |
| `actors/tiktok-comments-scraper` | `tiktok-comments-scraper` | `oaJANlheGg9o3EZjU` | TikTok Comments Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/tiktok-followers-scraper` | `tiktok-followers-scraper` | `1WE6uJzTx1DbS5u39` | TikTok Followers Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-21 (`PAY_PER_EVENT`); verify activation |
| `actors/tiktok-following-scraper` | `tiktok-following-scraper` | `a3CzWl85xlYKi9UIn` | TikTok Following Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-21 (`PAY_PER_EVENT`); verify activation |
| `actors/tiktok-hashtag-posts-scraper` | `tiktok-hashtag-posts-scraper` | `H2dZTreGZ7s3XJsQ7` | TikTok Hashtag Posts Scraper | Local source present; live Apify version uses `SOURCE_FILES`; latest run `9jme9tLGVrekgx8RW` succeeded; latest build `adY4fybFQ8GgZ2iXQ` succeeded | Active `PAY_PER_EVENT` pricing verified 2026-05-24T07:23:43.706Z from `pricingInfos` (`apify-default-dataset-item` at $0.0003/result); `pricingInfo/currentPricingInfo` still null |
| `actors/tiktok-profile-scraper` | `tiktok-profile-scraper` | `ElkkSkWZ7xAaOqsr4` | TikTok Profile Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-19 (`PAY_PER_EVENT`); verify activation |
| `actors/tiktok-search-scraper` | `tiktok-search-scraper` | `0h6AKrgNYjn7pM5EO` | TikTok Search Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Public with `pricingInfos` active from 2026-05-13 (`PAY_PER_EVENT`, default dataset item at $0.0002/result); verify `currentPricingInfo` if Apify API begins populating it |
| `actors/tiktok-video-scraper` | `tiktok-video-scraper` | `4DSOKG4JhcS4lhu60` | TikTok Video Details Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-21 (`PAY_PER_EVENT`, default dataset video detail result at $0.0002/result); verify activation |
| `actors/tiktok-user-posts-scraper` | `tiktok-user-posts-scraper` | `iSnxQQAvqnI0ZKL9F` | TikTok User Posts Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-19 (`PAY_PER_EVENT`); verify activation |
| `actors/trustedshops-search-scraper` | `trustedshops-search-scraper` | `m7Ss9UWPRN5cQhIAK` | Trusted Shops Search Scraper | Local source present; live Apify version uses `SOURCE_FILES`; cloud run `Aiz8AnjChhjer5rjV` succeeded with 20 results for `zalando` in `DEU` | Public with `pricingInfos` active from 2026-05-17 (`PAY_PER_EVENT`, `shop-result` at $0.0002/result); API still reports `pricingInfo/currentPricingInfo: null` immediately after publication |
| `actors/trustpilot-business-search-scraper` | `trustpilot-business-search-scraper` | `W8yULHo0Mzq7CYRrM` | Trustpilot Business Search Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-06-02 (`PAY_PER_EVENT`, `business-result` at $0.0002/result); verify activation |
| `actors/trustpilot-company-reviews-scraper` | `trustpilot-company-reviews-scraper` | `nBSazp2iBmBm1FQvz` | Trustpilot Company Reviews Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-23 (`PAY_PER_EVENT`); verify activation |
| `actors/vinted-search-scraper` | `vinted-search-scraper` | `u8F5YhfXkQIrgLe73` | Vinted Search Scraper | Local source present; live Apify version uses `SOURCE_FILES`; cloud run `SdU2bgbGDHeEilac1` succeeded with 1 result for `zara` in `FR` | Scheduled 2026-06-01 (`PAY_PER_EVENT`, `item-result` at $0.0002/result); Apify blocks immediate pricing with two-week lead time |
| `actors/website-content-extractor-scraper` | `website-content-extractor-scraper` | `bAUjzVta9rv6bpa10` | Website Content Extractor Scraper | Local source present; live Apify version uses `SOURCE_FILES`; build `f4O4HmNC1l6Ve3a06` / `1.0.2` succeeded; smoke run `74zT24s0Z5G0zUR7x` succeeded with 1 dataset item for `https://example.com` | Public with `PAY_PER_EVENT` pricing scheduled for 2026-06-09T19:25:00.000Z, `url-result` at $0.0002/result; Apify blocked immediate pricing with two-week lead time |
| `actors/youtube-api-batch-videos` | `youtube-api-batch-videos` | `6ZUj6u4SWuJxOQnn9` | YouTube API Batch Videos | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-channel-podcasts` | `youtube-api-channel-podcasts` | `Y3mKYlGNhsrBE7aZO` | YouTube API Channel Podcasts | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-channel-videos` | `youtube-api-channel-videos` | `w464EbPGGZqcmrC8j` | YouTube API Channel Videos | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-get-channel-about-details` | `youtube-api-get-channel-about-details` | `vKqlzEXa47Ubpuix5` | YouTube API Get Channel "About" Details | Local source imported from live Apify `SOURCE_FILES` on 2026-05-24 | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-get-channel-community` | `youtube-api-get-channel-community` | `S9Gf6PSqzz6hxvMNA` | YouTube API Get Channel Community | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-get-channel-details` | `youtube-api-get-channel-details` | `svtEvWGEssObsU72e` | YouTube API Get Channel Details | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-get-channel-livestreams` | `youtube-api-get-channel-livestreams` | `WT3XhaJ0lUYmp0eFu` | YouTube API Get Channel Livestreams | Local source imported from live Apify `SOURCE_FILES` on 2026-05-24 | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-get-channel-playlists` | `youtube-api-get-channel-playlists` | `3ERhmU2MUBjdR4AOq` | YouTube API Get Channel Playlists | Local source imported from live Apify `SOURCE_FILES` on 2026-05-24 | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-get-channel-shorts` | `youtube-api-get-channel-shorts` | `608amsD2lD6xRKbax` | YouTube API Get Channel Shorts | Local source imported from live Apify `SOURCE_FILES` on 2026-05-24 | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-get-channel-statistics` | `youtube-api-get-channel-statistics` | `yJiDippxXaK5hWQRC` | YouTube API Get Channel Statistics | Local source imported from live Apify `SOURCE_FILES` on 2026-05-24 | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-get-playlists-details` | `youtube-api-get-playlists-details` | `Xhwtx7clQKnPRez1H` | YouTube API Get Playlists Details | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-get-video-details` | `youtube-api-get-video-details` | `P1Jv1QuMoId4XUPlC` | YouTube Video Details Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-hashtags` | `youtube-api-hashtags` | `N5ol78TtqiMj4MtM6` | YouTube API Hashtags | Local source imported from live Apify `SOURCE_FILES` on 2026-05-24; live `actor.json.name` is stale (`my-actor`) | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-playlists` | `youtube-api-playlists` | `hueJrwkrbo20Ufrna` | YouTube API Playlists | Local source imported from live Apify `SOURCE_FILES` on 2026-05-24; live `actor.json.name` is stale (`thescrappa/unlimited-youtube-api`) | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-related-videos` | `youtube-api-related-videos` | `eKzA6GhEOJACIiCUW` | YouTube API Related Videos | Local source present; live Apify version uses `SOURCE_FILES` | Active `PAY_PER_EVENT` pricing verified 2026-05-21 |
| `actors/youtube-api-search-by-category` | `youtube-api-search-by-category` | `O1ltDU9qk4adR2x86` | YouTube API Search By Category | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-search-data` | `youtube-api-search-data` | `ziD2fUoLsdzKlc6zR` | YouTube Search Data Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-search-suggestions` | `youtube-api-search-suggestions` | `oecJ81oeff1KozCtd` | YouTube API Search Suggestions | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-trending-videos` | `youtube-api-trending-videos` | `T7ddx0tgVCwMHi9ET` | YouTube API Trending Videos | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-video-chapters` | `youtube-api-video-chapters` | `fq5Kq9OfBRWYu9go1` | YouTube API Video Chapters | Local source imported from live Apify `SOURCE_FILES` on 2026-05-24; live `actor.json.name` is stale (`my-actor`) | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-api-video-comments` | `youtube-api-video-comments` | `ZT2Z352FLhgqgtMrg` | YouTube Video Comments Scraper | Local source present; live Apify version uses `SOURCE_FILES` | Scheduled 2026-05-17 (`PAY_PER_EVENT`); verify activation |
| `actors/youtube-transcript-scraper` | `youtube-transcript-scraper` | `ztc698cHC09lkCDYE` | YouTube Transcript Scraper | Local source present; live Apify version uses `SOURCE_FILES`; Apify notice still `UNDER_MAINTENANCE`; latest run `hRCogd7KpVQU9oRJ5` succeeded | Scheduled 2026-05-19 (`PAY_PER_EVENT`); verify activation |

## API Key

All actors require a Scrappa API key. Get yours at [https://scrappa.co](https://scrappa.co).

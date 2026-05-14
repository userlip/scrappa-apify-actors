# Pricing Activation Audit - 2026-05-14

Verification timestamp: 2026-05-14T10:20:00Z.

Scope: live `TheScrappa` actors from `GET /v2/acts?limit=1000`, followed by actor detail checks through `GET /v2/acts/{actorId}`. Public actors were flagged when a paid `pricingInfos[]` row had `startedAt` at or before the verification timestamp but both `pricingInfo` and `currentPricingInfo` still returned `null`.

## Executive Summary

- 17 public `thescrappa` actors had due paid pricing rows at the requested checkpoint, but the Actor detail API still returned `pricingInfo: null` and `currentPricingInfo: null`.
- A follow-up live audit at 2026-05-14T10:25:57Z found an 18th newly due actor, `bSdKk0P65oTGDXLIh` / `google-patents-search-scraper`, after its scheduled pricing start at 2026-05-14T10:22:14Z.
- The affected set is monetization-critical because the active paid evidence fields remain empty after the scheduled start date, even though `pricingInfos[]` contains paid `PAY_PER_EVENT` pricing.
- A no-op API re-save was attempted for `zqckCpYtvPvxb2oxl` (`google-finance-historical-prices-scraper`) by PUTing its existing `pricingInfos[]` back to the actor. Apify accepted the update with HTTP 200, but the follow-up detail request still returned `pricingInfo: null` and `currentPricingInfo: null`.
- This should be escalated to Apify support as an Actor pricing activation/indexing bug. Console verification may still be useful, but the owner API evidence is currently inconsistent.

## Affected Actors

| Actor ID | Slug | Active paid pricingInfos | Active fields | Latest runs | Last build | SCRAPPA_API_KEY |
|---|---|---|---|---|---|---|
| `VZrsJ6bO3h92y0duj` | `instagram-user-info-cheapest-0-20-1000-results` | 2026-03-03T15:07:41.936Z; PAY_PER_EVENT `apify-default-dataset-item=$0.0002` | `pricingInfo=null`, `currentPricingInfo=null` | SUCCEEDED | `SOmcx1LwwOquSdmxJ` / 0.0.23 | present |
| `nfdzs1z0cRIU1Bfhw` | `instagram-post-info-cheapest-0-20-1000-results` | 2026-03-03T15:07:41.933Z; PAY_PER_EVENT `apify-default-dataset-item=$0.0002` | `pricingInfo=null`, `currentPricingInfo=null` | SUCCEEDED, SUCCEEDED, SUCCEEDED | `0CrSPsr153Sqlxarc` / 0.0.28 | present |
| `2pU7EbKhShUz8BAnN` | `google-search-scraper` | 2026-03-03T15:07:41.929Z; PAY_PER_EVENT `apify-default-dataset-item=tiered` | `pricingInfo=null`, `currentPricingInfo=null` | SUCCEEDED | `srtmkRpV5jPbBDYTe` / 1.0.8 | present |
| `87AaxKjjQrK0F0g60` | `linkedin-profile-scraper` | 2026-03-03T15:07:41.945Z; PAY_PER_EVENT `apify-default-dataset-item=$0.0003` | `pricingInfo=null`, `currentPricingInfo=null` | SUCCEEDED | `upz833R3XiSlsxACT` / 1.0.16 | present |
| `EMGCTVXuOBRERiDMf` | `linkedin-company-scraper` | 2026-03-03T15:07:41.939Z; PAY_PER_EVENT `apify-default-dataset-item=$0.0003` | `pricingInfo=null`, `currentPricingInfo=null` | No recent runs returned | `BKoBgEX6Daxwn3Yxt` / 1.0.17 | present |
| `hVDOXgRoKJbnATxzs` | `linkedin-post-scraper` | 2026-03-03T15:07:41.943Z; PAY_PER_EVENT `apify-default-dataset-item=$0.0003` | `pricingInfo=null`, `currentPricingInfo=null` | SUCCEEDED, FAILED | `REPabTYnendCvgSyc` / 1.0.10 | present |
| `3fXhf8bJruXVWgDKy` | `google-maps-search-scraper` | 2026-03-03T15:07:41.926Z; PAY_PER_EVENT `apify-default-dataset-item=$0.0003` | `pricingInfo=null`, `currentPricingInfo=null` | SUCCEEDED | `Y1MHQfAfq630bxIll` / 1.0.18 | present |
| `DT8bUdm2Vn4HjlyDo` | `google-maps-advanced-search-scraper` | 2026-03-15T23:00:00.000Z; PAY_PER_EVENT `search=$0.005`, `result=$0.0003` | `pricingInfo=null`, `currentPricingInfo=null` | SUCCEEDED | `agH9oDbqHRNZMKhIb` / 1.0.12 | present |
| `QvxzSeJiQrMggt1Vn` | `google-maps-reviews-scraper` | 2026-03-03T15:07:41.922Z; PAY_PER_EVENT `apify-default-dataset-item=$0.0003` | `pricingInfo=null`, `currentPricingInfo=null` | No recent runs returned | `p5HN5YWGKHpRQlGMZ` / 1.0.17 | present |
| `hhS8GkceJHFiexWe6` | `google-maps-autocomplete-scraper` | 2025-12-22T09:56:50.767Z; PAY_PER_EVENT `apify-actor-start=$0.00005`, `apify-default-dataset-item=$0.0003` | `pricingInfo=null`, `currentPricingInfo=null` | No recent runs returned | `9SbWxSeNdrsmvA6RP` / 1.0.7 | present |
| `JCqaAyY3Vy7K5UoRd` | `google-maps-business-details-scraper` | 2025-12-22T09:46:34.778Z; PAY_PER_EVENT `apify-actor-start=$0.00005`, `apify-default-dataset-item=$0.0003` | `pricingInfo=null`, `currentPricingInfo=null` | No recent runs returned | `yGD3HySCUNy92ZEW9` / 1.0.16 | present |
| `gLbfii9Nq4H7auMnN` | `google-maps-photos-scraper` | 2025-12-22T10:11:21.514Z; PAY_PER_EVENT `apify-actor-start=$0.00005`, `apify-default-dataset-item=$0.0003` | `pricingInfo=null`, `currentPricingInfo=null` | No recent runs returned | `Oc8vGxP3MvpVK2fXI` / 1.0.26 | present |
| `1D1neAFKb8LnbKvHG` | `google-trends-interest-scraper` | 2026-05-09T20:59:53.000Z; PAY_PER_EVENT `timeline-point=$0.0002` | `pricingInfo=null`, `currentPricingInfo=null` | SUCCEEDED, SUCCEEDED | `BfMSjuLWC616B74gA` / 1.0.2 | present |
| `OVlDREBAcO4iPyW64` | `indeed-jobs-scraper` | 2026-05-10T21:38:36.000Z; PAY_PER_EVENT `apify-default-dataset-item=$0.0003` | `pricingInfo=null`, `currentPricingInfo=null` | SUCCEEDED, SUCCEEDED | `CBftHnUYzRbIsIsTK` / 1.0.2 | present |
| `IIPXRhbeyXH7ssOK6` | `google-flights-search-scraper` | 2026-05-11T21:10:28.856Z; PAY_PER_EVENT `flight-result=$0.0002` | `pricingInfo=null`, `currentPricingInfo=null` | SUCCEEDED, SUCCEEDED, SUCCEEDED | `bN2aTiXpjvvynl2eI` / 1.0.3 | present |
| `0h6AKrgNYjn7pM5EO` | `tiktok-search-scraper` | 2026-05-13T13:08:08.000Z; PAY_PER_EVENT `apify-default-dataset-item=$0.0002` | `pricingInfo=null`, `currentPricingInfo=null` | SUCCEEDED, SUCCEEDED | `QEWgP1JmVRc3kgvhb` / 1.0.4 | present |
| `zqckCpYtvPvxb2oxl` | `google-finance-historical-prices-scraper` | 2026-05-13T14:04:27.000Z; PAY_PER_EVENT `price-point=$0.0002` | `pricingInfo=null`, `currentPricingInfo=null` | SUCCEEDED, FAILED, FAILED | `WSfvkon4YiGocXsuZ` / 1.0.1 | present |

## Newly Due During This Run

| Actor ID | Slug | Active paid pricingInfos | Active fields | Latest runs | Last build | SCRAPPA_API_KEY |
|---|---|---|---|---|---|---|
| `bSdKk0P65oTGDXLIh` | `google-patents-search-scraper` | 2026-05-14T10:22:14.548Z; PAY_PER_EVENT `apify-default-dataset-item=$0.0002` | `pricingInfo=null`, `currentPricingInfo=null` | SUCCEEDED | `VP3OOb7DCVUmVCVLJ` / 1.0.1 | present |

## Re-save Attempt

Actor tested: `zqckCpYtvPvxb2oxl` / `google-finance-historical-prices-scraper`.

Action:

```bash
PUT /v2/acts/zqckCpYtvPvxb2oxl
body: {"pricingInfos": <existing pricingInfos from GET /v2/acts/zqckCpYtvPvxb2oxl>}
```

Result:

- Apify returned HTTP 200.
- `modifiedAt` changed to `2026-05-14T10:20:13.462Z`.
- Follow-up `GET /v2/acts/zqckCpYtvPvxb2oxl` still returned `pricingInfo: null` and `currentPricingInfo: null`.
- `pricingInfos[]` remained present with paid `PAY_PER_EVENT` pricing started at `2026-05-13T14:04:27.000Z`.

## Support Escalation Payload

Ask Apify support to investigate why the owner Actor detail API does not populate active pricing fields after paid pricing start.

Summary:

```text
TheScrappa has 17 public actors at the 2026-05-14T10:20:00Z checkpoint where paid pricingInfos have started, but GET /v2/acts/{actorId} still returns pricingInfo=null and currentPricingInfo=null. A no-op pricingInfos re-save on zqckCpYtvPvxb2oxl returned HTTP 200 and updated modifiedAt, but the active pricing fields remained null. A follow-up audit at 2026-05-14T10:25:57Z shows 18 overdue actors after bSdKk0P65oTGDXLIh reached its scheduled start.
```

Include these examples:

- `zqckCpYtvPvxb2oxl` / `google-finance-historical-prices-scraper`: `pricingInfos[0].startedAt=2026-05-13T14:04:27.000Z`, `PAY_PER_EVENT price-point=$0.0002`, `pricingInfo=null`, `currentPricingInfo=null` after re-save.
- `IIPXRhbeyXH7ssOK6` / `google-flights-search-scraper`: `pricingInfos[].startedAt=2026-05-11T21:10:28.856Z`, `PAY_PER_EVENT flight-result=$0.0002`, `pricingInfo=null`, `currentPricingInfo=null`.
- `0h6AKrgNYjn7pM5EO` / `tiktok-search-scraper`: `pricingInfos[].startedAt=2026-05-13T13:08:08.000Z`, `PAY_PER_EVENT apify-default-dataset-item=$0.0002`, `pricingInfo=null`, `currentPricingInfo=null`.
- `bSdKk0P65oTGDXLIh` / `google-patents-search-scraper`: `pricingInfos[].startedAt=2026-05-14T10:22:14.548Z`, `PAY_PER_EVENT apify-default-dataset-item=$0.0002`, `pricingInfo=null`, `currentPricingInfo=null`.

Question for Apify:

```text
Are these actors billable despite pricingInfo/currentPricingInfo being null in the owner Actor detail API? If yes, what API field should be treated as authoritative for active paid pricing? If no, please activate or repair the current pricing records for the listed public actors.
```

## Verification Follow-up

- Re-run `APIFY_TOKEN=... pnpm audit:pricing --now 2026-05-14T10:20:00.000Z`.
- If `pnpm` is unavailable in the workspace, run the underlying script directly: `APIFY_TOKEN=... node scripts/audit-apify-pricing.mjs --now 2026-05-14T10:20:00.000Z`.
- For each support-fixed actor, confirm `pricingInfo` or `currentPricingInfo` returns the active paid row, or obtain written Apify confirmation that `pricingInfos[]` is the authoritative billable field.
- Re-run the activation checkpoint after the next scheduled pricing date: `APIFY_TOKEN=... pnpm audit:pricing --now 2026-05-17T15:00:00.000Z`.

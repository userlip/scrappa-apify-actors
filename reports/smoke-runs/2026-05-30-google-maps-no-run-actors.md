# 2026-05-30 Google Maps No-Run Actor Smoke Sweep

Scope: smoke-run the six public paid Google Maps actors that were reported as `NO_RUNS` in the owner-filtered Apify health audit. Inputs were intentionally small and routed through Scrappa API endpoints; no actor code fixes were required.

Post-run validation: `APIFY_TOKEN=... node scripts/audit-apify-health.mjs --json` at `2026-05-30T13:18:03.527Z` reported all six target actors as `OK`.

| Actor | Actor ID | Pricing status | Secret check | Latest build | Smoke input | Run ID | Status | Default dataset items | Log evidence |
| --- | --- | --- | --- | --- | --- | --- | --- | ---: | --- |
| `google-maps-search-scraper` | `3fXhf8bJruXVWgDKy` | `PRICE_PER_DATASET_ITEM` | `SCRAPPA_API_KEY` present as secret | `Y1MHQfAfq630bxIll` (`SUCCEEDED`) | `query: Starbucks Times Square New York`, `hl: en`, `gl: us`, `use_cache: true`, `maximum_cache_age: 604800`, `fallback_zoom: 13` | `g7kzh2ruXr4DE83M9` | `SUCCEEDED` | 22 | Log reported `Found 22 results` and `Search completed successfully`. First result supplied business ID `0x89c258552071bcb3:0x5cc7129cc313de1a` for entity smoke runs. |
| `google-maps-advanced-search-scraper` | `DT8bUdm2Vn4HjlyDo` | `PRICE_PER_DATASET_ITEM` | `SCRAPPA_API_KEY` present as secret | `agH9oDbqHRNZMKhIb` (`SUCCEEDED`) | `query: coffee near Times Square New York`, `zoom: 16`, `latitude: 40.758`, `longitude: -73.9855`, `limit: 1`, `hl: en`, `gl: us` | `i0hesdsBqSkIF7qYt` | `SUCCEEDED` | 1 | Log reported `Found 1 results` and an advanced-search summary with `results_found: 1`. |
| `google-maps-autocomplete-scraper` | `hhS8GkceJHFiexWe6` | `PAY_PER_EVENT` | `SCRAPPA_API_KEY` present as secret | `9SbWxSeNdrsmvA6RP` (`SUCCEEDED`) | `query: coffee times square` | `AXpBYUsbYmIFIzY2g` | `SUCCEEDED` | 5 | Log reported `Found 5 suggestions` and `Autocomplete completed`. |
| `google-maps-business-details-scraper` | `JCqaAyY3Vy7K5UoRd` | `PAY_PER_EVENT` | `SCRAPPA_API_KEY` present as secret | `yGD3HySCUNy92ZEW9` (`SUCCEEDED`) | `business_id: 0x89c258552071bcb3:0x5cc7129cc313de1a`, `use_cache: true`, `maximum_cache_age: 604800` | `O5e92doO3LZ3AI8Sj` | `SUCCEEDED` | 1 | Log reported `Successfully fetched: Starbucks Coffee Company` and `Completed successfully`. |
| `google-maps-photos-scraper` | `gLbfii9Nq4H7auMnN` | `PAY_PER_EVENT` | `SCRAPPA_API_KEY` present as secret | `Oc8vGxP3MvpVK2fXI` (`SUCCEEDED`) | `business_id: 0x89c258552071bcb3:0x5cc7129cc313de1a`, `use_cache: true`, `maximum_cache_age: 604800` | `ZC0Qa2pJSyqcs7A2Y` | `SUCCEEDED` | 10 | Log reported `Found 10 photos` and `Photos extraction completed`. |
| `google-maps-reviews-scraper` | `QvxzSeJiQrMggt1Vn` | `PRICE_PER_DATASET_ITEM` | `SCRAPPA_API_KEY` present as secret | `p5HN5YWGKHpRQlGMZ` (`SUCCEEDED`) | `business_id: 0x89c258552071bcb3:0x5cc7129cc313de1a`, `sort: 2`, `limit: 1`, `use_cache: true`, `maximum_cache_age: 604800` | `ogKgPIXNVm4l8fT72` | `SUCCEEDED` | 1 | Log reported `Found 1 reviews`, `Google Maps Reviews extraction completed successfully`, and `has_next_page: true`. |

Notes:

- Monetization was checked before smoke runs through the Apify Actor detail API. All six actors had active paid pricing; no pricing changes were needed.
- Version `1.0` environment variables were checked before smoke runs. All six actors had `SCRAPPA_API_KEY` configured as an Apify secret.
- Dataset counts were rechecked after run completion through each default dataset endpoint because immediate post-run metadata lagged briefly.
- No wrapper validation, optional boolean/string, secret, or deployment regression was found. No branch, code fix, deploy, or PR was needed.

# Public Actor Smoke Evidence - 2026-05-22

Task: smoke-run the six public TheScrappa actors flagged as having successful builds but no recorded cloud-run evidence in the sweep, then record dataset/log evidence before listing promotion.

Live Apify stats checked on 2026-05-22 showed no remaining public TheScrappa actor with `totalRuns=0`; the six newest/lowest-history public wrappers from the sweep were smoke-run anyway.

## Actors Smoke-Run

| Actor ID | Slug | Build ID | Pricing status | Memory / timeout | Smoke run | Status | Dataset evidence | Log evidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `MDgsOkRoh1bAfC28g` | `similarweb-traffic-analytics-scraper` | `g13DCEkNQUTP4xTSo` | Scheduled `PAY_PER_EVENT` from 2026-06-04, event `domain-result` | 128 MB / 300s | `iWHaW8MohWeKJTHZe` | `SUCCEEDED` | Dataset `yKkuhi8v1FQeepSRE`, 1 checked item; keys include `success`, `domain`, `site_name`, `visits`, `traffic_direct` | `Results summary: {"requested":1,"processed":1,"successful":1,"no_data":0}` |
| `4DSOKG4JhcS4lhu60` | `tiktok-video-scraper` | `9Qq5WduPyNXnQa7mf` | Scheduled `PAY_PER_EVENT` from 2026-05-21, event `apify-default-dataset-item` | 128 MB / 300s | `JGhArekpv1LG06Qv7` | `SUCCEEDED` | Dataset `tPX7TCLK8eby93clK`, 1 checked item; keys include `aweme_id`, `id`, `region`, `title`, `play_count` | `Results summary: {"urls_requested":1,"dataset_items":1,"videos_found":1,"lookups_failed":0,"hd_requested":false}` |
| `BehWN3LEvBxhEiJDF` | `booking-search-scraper` | `0O8qenLBUpr15BK3d` | Scheduled `PAY_PER_EVENT` from 2026-06-03, event `booking-result` | 256 MB / 360s | `N3nanH7zuJ8K9eyrf` | `FAILED` | Dataset `MYgABxlcv5veXXD13`, 0 checked items | Fails after three Scrappa API attempts: `Scrappa API error (502): Bad Gateway` |
| `W8yULHo0Mzq7CYRrM` | `trustpilot-business-search-scraper` | `ScrfclecE2klhh16v` | Scheduled `PAY_PER_EVENT` from 2026-06-02, event `business-result` | 256 MB / 360s | `5qAVI1ekzorKqVhTS` | `SUCCEEDED` | Dataset `2qbobC3v2IbbweccH`, 3 checked items from 4 saved; keys include `businessUnitId`, `displayName`, `identifyingName`, `trustScore`, `business_name` | `Results summary: {"search_type":"company_search","pages_fetched":1,"businesses_extracted":4,"total_results":372,"total_pages":75}` |
| `8SvzPgdsdg1yZK1t4` | `jameda-search-scraper` | `UA2Pp5PJ4XTlWzjob` | Scheduled `PAY_PER_EVENT` from 2026-06-02, event `doctor-result` | 256 MB / 360s | `XXhrNyX2Vlr9gbyGw` | `SUCCEEDED` | Dataset `74urUsFnJdaqg8QYG`, 1 checked item; keys include `id`, `name`, `url`, `specialization`, `rating` | `Results summary: {"pages_fetched":1,"responses_saved":1,"doctors_extracted":1,"total_results":3998,"total_pages":3998}` |
| `u8F5YhfXkQIrgLe73` | `vinted-search-scraper` | `I4gRBhrQMVz5dbEBo` | Scheduled `PAY_PER_EVENT` from 2026-06-01, event `item-result` | 256 MB / 420s | `aoWqxQMnY0QqOa1PI` | `SUCCEEDED` | Dataset `4WoQtTqkob1OTe4yS`, 1 checked item; keys include `id`, `title`, `price`, `brand_title`, `url` | `Results summary: {"pages_fetched":1,"responses_saved":1,"items_extracted":1,"total_pages":960,"total_entries":960}` |

All six actors have a `SCRAPPA_API_KEY` env var configured as an Apify secret on version `1.0`; Apify does not expose secret values through the API.

## Inputs Used

```json
{
  "similarweb-traffic-analytics-scraper": {"domains": ["google.com"]},
  "tiktok-video-scraper": {"urls": ["https://www.tiktok.com/@tiktok/video/7568510388342443294"]},
  "booking-search-scraper": {
    "ss": "Paris",
    "checkin": "2026-06-01",
    "checkout": "2026-06-04",
    "group_adults": 2,
    "group_children": 0,
    "no_rooms": 1,
    "currency": "EUR",
    "lang": "en-us"
  },
  "trustpilot-business-search-scraper": {
    "search_type": "company_search",
    "query": "google",
    "locale": "en-US",
    "page": 1,
    "max_pages": 1,
    "per_page": 5
  },
  "jameda-search-scraper": {"q": "Zahnarzt", "loc": "Berlin", "page": 1, "per_page": 1, "max_pages": 1},
  "vinted-search-scraper": {"query": "nike shoes", "country": "US", "page": 1, "per_page": 1, "max_pages": 1, "order": "relevance"}
}
```

## Additional Run Notes

- Booking was retried three times during this cycle:
  - `Z9Th0fZxFRvOI7a0t`: Berlin without dates, failed with Scrappa API `502`.
  - `PFyvwNMAuiXH2K4mg`: Berlin with future dates, failed with Scrappa API `502`.
  - `N3nanH7zuJ8K9eyrf`: exact input from prior successful run `fpRPbhoYDIlB7SaKf`, failed with Scrappa API `502`.
- Prior Booking evidence exists from 2026-05-21: run `fpRPbhoYDIlB7SaKf` succeeded, dataset `z51STfH5hydG36rVA` had Booking result keys including `name`, `url`, `review_score`, `price`, and request metadata.
- Trustpilot first smoke input `jmfzYxmZgdxijsHLf` succeeded but saved 0 rows for `amazon` with `country=US`; retry `5qAVI1ekzorKqVhTS` saved 4 rows for `google`.

## Data Sources

- Apify actor metadata API: `GET /v2/acts/{actorId}` for all six actors.
- Apify version API: `GET /v2/acts/{actorId}/versions/1.0` to verify `SCRAPPA_API_KEY` presence.
- Apify run API: `POST /v2/acts/{actorId}/runs?waitForFinish=120` for smoke runs.
- Apify run logs: `GET /v2/actor-runs/{runId}/log`.
- Apify datasets: `GET /v2/datasets/{datasetId}/items?clean=true&limit=3`.
- Local schemas read from `actors/*/.actor/input_schema.json`.

## Recommendation

Hold listing promotion for `booking-search-scraper` until the Scrappa Booking endpoint stops returning `502` for the previously successful Paris input. The other five actors have fresh successful cloud-run evidence with dataset rows and scheduled paid per-event pricing.

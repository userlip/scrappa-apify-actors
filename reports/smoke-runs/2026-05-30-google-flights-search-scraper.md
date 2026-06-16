# Google Flights Search Scraper Smoke Evidence - 2026-05-30

Task: smoke-run the public paid TheScrappa actor `google-flights-search-scraper` after the health sweep found no prior run history, while preserving Scrappa-side scraping and paid Apify monetization.

## Result

| Actor ID | Slug | Build ID / number | Pricing status | Memory / timeout | Smoke run | Status | Dataset evidence | Log evidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `IIPXRhbeyXH7ssOK6` | `google-flights-search-scraper` | `bN2aTiXpjvvynl2eI` / `1.0.3` | Active `PAY_PER_EVENT` evidence from `pricingInfos`, event `flight-result` at `$0.0002/result`, started `2026-05-11T21:10:28.856Z` | 256 MB / 240s | `yCqbYvZVyXyi7bEGs` | `SUCCEEDED` | Dataset `1V5bLAtasXuVq2Or4`, 31 items; checked sample fields include `position`, `trip_type`, `price`, `currency`, `stops`, `airline_names`, `flight_numbers`, `departure_airport`, `arrival_airport`, `booking_token`, and request metadata | `Results summary: {"trip_type":"one_way","origin":"JFK","destination":"LAX","departure_date":"2026-09-15","return_date":null,"flights":31,"has_baggage_info":false}` |

## Input Used

```json
{
  "trip_type": "one_way",
  "origin": "JFK",
  "destination": "LAX",
  "departure_date": "2026-09-15",
  "adults": 1,
  "cabin_class": "economy",
  "currency": "USD",
  "hl": "en",
  "gl": "us",
  "include_baggage": false
}
```

## Run Notes

- Run `yCqbYvZVyXyi7bEGs` started at `2026-05-29T22:16:24.095Z` and finished at `2026-05-29T22:16:28.043Z`.
- Platform usage was `0.0002640972222222222` compute units with `usageTotalUsd` about `$0.0003135`.
- The wrapper called the Scrappa Google Flights endpoint and wrote one dataset item per returned flight result. No Scrappa scraping workload was moved into Apify.
- Version `1.0` still references `SCRAPPA_API_KEY` as an Apify secret. Apify does not expose secret values through the API.
- Direct actor metadata still reports `pricingInfo: null` and `currentPricingInfo: null`, but the repository pricing audit treats the started paid `pricingInfos` entry as active paid evidence.

## Pricing Audit

Post-run live pricing audit was run with `node scripts/audit-apify-pricing.mjs` at `2026-05-29T22:17:18.472Z`.

Summary:

- Public actors checked: 68
- Active paid pricing evidence: 55
- Overdue missing active pricing: 0
- Missing paid pricing: 0
- Future-only paid pricing: 13
- Actor audit errors: 0

## Data Sources

- Local schema and README: `actors/google-flights-search-scraper/.actor/input_schema.json`, `actors/google-flights-search-scraper/README.md`
- Apify actor metadata: `GET /v2/acts/IIPXRhbeyXH7ssOK6`
- Apify recent runs: `GET /v2/acts/IIPXRhbeyXH7ssOK6/runs`
- Apify smoke run: `POST /v2/acts/IIPXRhbeyXH7ssOK6/runs`
- Apify run detail: `GET /v2/actor-runs/yCqbYvZVyXyi7bEGs`
- Apify run log: `GET /v2/actor-runs/yCqbYvZVyXyi7bEGs/log`
- Apify dataset: `GET /v2/datasets/1V5bLAtasXuVq2Or4` and `GET /v2/datasets/1V5bLAtasXuVq2Or4/items?clean=true&limit=3`
- Apify version env vars: `GET /v2/acts/IIPXRhbeyXH7ssOK6/versions/1.0`

## Recommendation

Keep `google-flights-search-scraper` promotion-eligible. It now has successful first-run cloud evidence, paid per-result pricing evidence, a successful latest build, non-empty dataset output, and clean logs.

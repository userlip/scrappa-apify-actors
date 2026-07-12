# Google Finance Indices Scraper implementation

Added `actors/google-finance-indices-scraper` on `feat/google-finance-indices-scraper`.

- Thin 128 MB / 120-second Apify wrapper for Scrappa's verified `/api/google-finance/indices` route.
- Supports a JSON array or CSV batch input, normalizes and deduplicates up to 50 symbols, and validates `hl`/`gl`.
- Emits at most one canonical dataset item per returned index and charges `index-result` only after a successful save. Case-insensitive upstream matching, output-ID deduplication, unmatched-row rejection, and charge-limit handling avoid duplicate or invalid charges.
- Listing documentation covers S&P 500, Dow, NASDAQ, custom symbols, pricing ($0.00025/result), and the direct Scrappa API path.

Validated locally from the actor directory:

```text
npm test                 # 5 passing
npm run typecheck        # passing
npx apify-cli validate-schema  # input and dataset schemas valid
jq empty .actor/actor.json .actor/input_schema.json
git diff --check main...HEAD
```

Deployment, secret configuration, Apify pricing activation, publication, and live smoke verification remain release-stage work; no credentials were available locally and no external state was changed.

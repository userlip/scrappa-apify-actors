# Implementation Summary: Google Finance Indices Scraper

Date: 2026-07-13

Branch: `fix/google-finance-indices-charge-limit-tests`

## Change completed

Removed the unnecessary `OUTPUT` key-value-store write from `actors/google-finance-indices-scraper`.

The Actor continues to write one dataset item per successfully saved, unique index and to charge only successful `index-result` dataset writes. The compact batch summary remains in logs, avoiding an extra storage operation in this thin Scrappa wrapper.

Added focused regression coverage for the PAY_PER_EVENT boundary: zero charge capacity does not fetch or write; a refused chargeable write is neither saved nor charged; and a final successful charged write is retained exactly once while later rows are not attempted. Direct `saveIndex` tests now cover both non-PPE dataset writes and accepted/refused PPE outcomes.

Added `actors/google-finance-indices-scraper/.actor/README.md` for the Apify marketplace listing. It names the Actor, documents batch input and S&P 500, Dow, NASDAQ, and custom-symbol use cases, explains the upstream matching caveat, and states the dataset-only `$0.00025` `index-result` charging model and direct Scrappa API upgrade path.

## Review follow-up completed

The Scrappa endpoint accepts a comma-separated value but returns only one index row. The Actor now makes one bounded, concurrent Scrappa request per normalized requested symbol within the same Apify run, rather than silently reducing a batch to its first result. The input cap is three symbols, which keeps three independent 90-second retry budgets within the 120-second Actor timeout when run concurrently and preserves the shutdown reserve.

Each response is matched to the exact symbol that initiated its request. A mismatched or failed request is logged as an uncharged failure; matching rows are aggregated, deduplicated, written once to the dataset, and charged only after Apify accepts the `index-result` event. Regression tests cover `.INX`, `.DJI`, and `.IXIC` per-symbol requests, aggregation, mismatch rejection, and partial request failure without charging the failed symbol. The marketplace README was updated to reflect the three-symbol limit and per-symbol execution model.

## Verification

From `actors/google-finance-indices-scraper`:

```text
npm test                              # 18 focused Actor tests pass
npm run typecheck                     # passes
npx apify-cli validate-schema         # input and embedded dataset schemas pass
jq empty .actor/actor.json .actor/input_schema.json  # passes
npm audit --omit=dev --audit-level=high              # no vulnerabilities
git diff --check                      # passes
```

No deployment, pricing, secret, or public-listing mutation was performed in this implementation stage. Those release checks remain for the deployment and live-verification stages.

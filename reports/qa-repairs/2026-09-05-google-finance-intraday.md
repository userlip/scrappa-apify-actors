# Google Finance Intraday QA repair

Mimir task: `a7db9f64-fdfc-4266-8950-8497df85fdd5`
Actor: `thescrappa/google-finance-intraday-scraper` (`IeOWC1bvG8SUebyDa`).

## Failure and cause

Apify QA run [`efHjm0AGPchmgVJvn`](https://console.apify.com/view/runs/efHjm0AGPchmgVJvn) failed on September 4, 2026 after 27.365 seconds. Build `1.0.2` exhausted three retries on HTTP 503. Its input exactly matches the current prefill and defaults:

```json
{"symbols":[{"symbol":"AAPL","exchange":"NASDAQ"}],"hl":"en","gl":"us"}
```

Direct API checks also returned HTTP 503 for MSFT, GOOGL, and BTC-USD. A fresh AAPL page fetched through Scrappa's existing provider contained 391 minute-level points in `ds:11`; `ds:10` had become quote-summary data. Scrappa's parser read only `ds:10` and classified the page as unrecognized.

The upstream repair preserves the legacy store when it contains recognized points and otherwise tries `ds:11`: [Scrappa PR #5930](https://github.com/userlip/scrappa/pull/5930). The actor client now includes Scrappa's `error` response field in error messages rather than hiding the actual error behind `Service Unavailable`.

The prefill, batching, price-point billing, retry limits, 128 MB memory setting, and automatic testing remain unchanged. No synthetic results or success-on-503 behavior was introduced.

## Validation

- Upstream: 20 parser tests and 7 intraday controller tests passed; Pint passed.
- Full captured AAPL response: `valid_graph`, 391 points after the parser fix (zero before).
- Actor: 13 tests and TypeScript typecheck passed.
- Backend PR #5930 merged as `913868fc9e17ea8a065207027349b75cb58a85c6`; all 15 CI checks passed after retrying a missing-Docker runner and an unrelated deployment-script timing test.
- Actor PR #304: all 29 checks passed for the implementation commit.
- Published Apify build: `1.0.3` (`7CsGkRguSHzXkO9WO`), tagged `latest`.

| Run | Build selection | Status | Start-to-finish time | Dataset items |
| --- | --- | --- | --- | --- |
| [ZqrArsY3CZqFcZic1](https://console.apify.com/view/runs/ZqrArsY3CZqFcZic1) | Candidate `qa-repair` | SUCCEEDED | 10.053 seconds | 391 |
| [8nK8CvXhcaRR0R4Sv](https://console.apify.com/view/runs/8nK8CvXhcaRR0R4Sv) | Published `latest` | SUCCEEDED | 32.835 seconds | 391 |

Both runs used the exact failed-QA input, derived from the deployed schema's prefill/defaults and compared with the original INPUT record. Every returned item had symbol AAPL, exchange NASDAQ, a numeric price, and a parseable timestamp. Both OUTPUT summaries reported one succeeded request, zero failures/no-data requests, and 391 graph points matching the dataset. The published-run dataset is `EsUS3BzSaaF0p11IK`.

After successful verification, the maintenance notice was cleared and read back as `null`. Automatic testing remains enabled. The temporary candidate tag was removed; `latest` points to the verified build.

## Rollout notes

An early candidate run, [HiGTbjfhDfM0k4SLm](https://console.apify.com/view/runs/HiGTbjfhDfM0k4SLm), still returned 503 during the partial backend rollout. It is not counted as a successful verification.

The initial guarded rollout updated four origins, then stopped because the production branch advanced. The subsequent release `26ca2278d166969bd0002f7bf8a27af622465f4f` includes this fix and the independently merged PR #5931, with all 15 checks green. Its already-running guarded rollout updated all five origins before the successful actor runs. That separate rollout was still checking origin readiness at the time of actor verification; no parallel deployment or manual readiness override was started by this repair.

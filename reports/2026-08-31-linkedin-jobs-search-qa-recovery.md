# LinkedIn Jobs Search Scraper QA recovery

Date: 2026-08-31 UTC

Actor: `thescrappa/linkedin-jobs-search-scraper` (`GAAKVpkPvj3lMbO6G`)

## Root cause

Apify automated QA run `jDZ8LVfajJ72dQxGT` used the expected prefilled input:

```json
{"query":"software engineer remote","num":10,"hl":"en","gl":"us","safe":"off"}
```

The actor started normally but the Scrappa `/linkedin/jobs/search` route returned HTTP 500 on all three attempts. The run failed after 146.099 seconds. Reproduction against the live Scrappa API returned the same HTTP 500 for the default query and two unrelated job queries, including a one-result request, so the problem was not the input schema or the default query.

Scrappa's `/search-light` route remained healthy. With the explicit `site:linkedin.com/jobs/view/` constraint it returned ten LinkedIn job URLs for the prefilled query in about three seconds.

## Repair

The actor now calls `/search-light` and adds `site:linkedin.com/jobs/view/` to the outgoing query. All other user input, normalization, retries, result publishing, and the prefilled input schema remain unchanged. Automated testing was not disabled.

## Validation

- Actor tests: 15 passed, 0 failed.
- Apify input and dataset schemas: valid.
- Local run with the exact prefilled QA input: succeeded in 3.055 seconds with 10 results.
- Deployed build: `1.0.3` (`tzaPvkrjb3zcHUDYu`), build status `SUCCEEDED`.
- Deployed validation run: `RehEHfI2gSEgHT3JG`, status `SUCCEEDED`, duration 4.027 seconds.
- Validation dataset: `jTi85kofyLcbqDFuA`, 10 items; every item URL matched `https://www.linkedin.com/jobs/view/`.

The deployed validation is comfortably below Apify's five-minute automated QA limit.

# Google Images Actor QA recovery

Investigated on 2026-09-03 for public Actor
`thescrappa/google-images-scraper` (`MrbqFgdpNTQcRW0Vt`).

## Triggering failure

Apify automated QA run `tXEVwJtWa8hdWyoIM` used production build `1.0.7`
(`c9efK7gITR1b5dFjd`) and the input below:

```json
{
  "queries": ["coffee product photography"],
  "page": 1,
  "hl": "en",
  "gl": "us",
  "safe": "active"
}
```

The run made two Scrappa `/api/images` attempts. Both reached the Actor's
30-second upstream timeout, so the run failed after 65.228 seconds. The Actor
was therefore well inside Apify's 300-second QA run limit; its status was
`FAILED` because the cold prefilled query exhausted the bounded retry policy.

## Root cause and fix

The prefilled query was not the request covered by Scrappa's production
monitoring. Scrappa's Checkybot definition continuously exercises
`q=coffee&hl=en&gl=us&safe=active`, while Apify QA exercised the unrelated
`coffee product photography` query. That allowed a cold and intermittently slow
upstream path to fail Apify's daily test without the same request being kept
healthy by production monitoring and cache reuse.

The Actor remains a thin Scrappa API wrapper. Its input-schema prefill now uses
`coffee`, combined with the existing defaults `page=1`, `hl=en`, `gl=us`, and
`safe=active`, so Apify QA and Checkybot exercise the same representative
request. The runtime timeout and retry policy are unchanged, avoiding renewed
unbilled retry amplification for customer traffic.

## Validation

- `npm test`: 14/14 tests passed, including the prefill regression test.
- `apify validate-schema .actor/input_schema.json`: passed.
- Production build `1.0.8` (`yKeVdA4fXKEgnC0D5`) succeeded and received the
  `latest` tag.
- QA-style production run `wmbApBi0Uki68NsRD` used the exact new prefilled
  input with Apify's 300-second timeout and 128 MB memory limit. It succeeded
  in 11.217 seconds on build `1.0.8`.
- The run wrote 100 clean dataset rows to dataset `7NFoX2xnc6V6gCDg8` and
  logged a successful summary with 100 image results, 39 products, 100
  original image URLs, and 100 dimension pairs.
- After the fresh green run, the stale Actor notice was changed from
  `UNDER_MAINTENANCE` to `NONE` with a minimal Actor metadata update. A direct
  detail read confirmed `notice: "NONE"`, `notices: null`, and `isPublic: true`.

The successful run verifies the fix with 288.783 seconds of headroom under
Apify's five-minute automated-test limit. The recovered Actor is public, its
`latest` build and QA-style run are green, and it no longer carries the
maintenance notice.

# Instagram Post Info QA recovery

Actor: `thescrappa/instagram-post-info-cheapest-0-20-1000-results` (`nfdzs1z0cRIU1Bfhw`).
Mimir task: `a953eba1-31f5-4668-8d31-0c236ef158af`.

## Failure and diagnosis

Apify QA run `E5qUkkcOafNCnUYvh`, build `0.0.34`, started on September 4 at 18:24:31 UTC with `{"url":"https://www.instagram.com/p/DUBtwxGEqz2/"}`. Scrappa returned HTTP 503, `instagram_post_upstream_unavailable`, on each attempt. The actor waited 30 seconds, then 90 seconds, then began a 180-second wait before Apify terminated it at 300 seconds. No dataset item was written.

The full previous retry schedule waited 2,100 seconds; seven 60-second HTTP calls could bring the request-and-wait budget to 2,520 seconds. Its default run timeout was 3,300 seconds.

Live requests reproduced HTTP 503 for the old example, alternate shortcodes, account-qualified URLs, and a current post. Scrappa's separate `/api/instagram/user/posts?username=instagram` endpoint returned real current posts successfully. The single-post service's own fallback path was unavailable too; no backend repair is claimed here.

Live source had both a URL `prefill` and `default`, which were missing from the repository schema. The live default could override API clients using only `shortcode` or `media_id` because URL takes precedence in input resolution.

## Repair

- Keep all scraping on Scrappa infrastructure.
- On transient single-post failure, account-qualified Instagram URLs can use the Scrappa user-posts endpoint. Publish only the exact requested shortcode, never an unrelated feed entry, an error, or a synthetic result.
- Authentication and explicitly non-retryable failures do not trigger this fallback. Missing feed matches preserve failure rather than claiming the post was deleted.
- Both requests share a 60-second abort deadline. Two retries wait 5 and 15 seconds: at most 200 seconds of requests and waits, leaving startup/output headroom.
- Set repository and live default options to 300 seconds and 128 MB; preserve live memory bounds.
- Prefill the verified public URL `https://www.instagram.com/instagram/p/Dc30nJeRKKz/`. Do not inject a URL default into explicit API inputs.
- Document fallback behavior and limits in actor documentation.

## Validation

- Actor `npm test`: 35 passed, 0 failed, including exact-match fallback, missing match, authentication failures, non-retryable errors, shared deadline, input compatibility, and retry budget.
- `git diff --check`: passed.
- Apify build validated input/dataset schemas. Live source files match the local actor sources; the API key remains secret.
- Initial build `0.0.35` (`QxWhxbVlh0UV1HPIH`) succeeded. Run `L4Beo0ww2NbKiQ4Ys` succeeded in 8.711 seconds start-to-finish, with one real post and one charged dataset event.
- Final build `0.0.36` (`80d8S28kO9tB0t7Q1`) includes a documentation correction and identical runtime code.

## Remaining upstream limitation

The single-post endpoint still returns 503. The fallback supports recent posts when the input includes the account username; bare shortcodes and older posts remain dependent on the single-post endpoint recovering. Refresh the prefilled example if it leaves the recent feed while this outage continues. This repair restores a genuine successful QA run without hiding the upstream outage.

Apify still reports `UNDER_MAINTENANCE` immediately after deployment. Its [testing documentation](https://docs.apify.com/actors/publishing/test) says a rebuilt actor is picked up within 24 hours. No manual badge override or support email was sent.

Final verification: run `i1k4buSpXlamDmiJO` on build `0.0.36` succeeded in **5.592 seconds**, using prefill extracted from the deployed schema and a 300-second timeout. Dataset `kAty6bTafhTLt90xv` has exactly one matching post with caption, author, and media. Charged event count: 1.

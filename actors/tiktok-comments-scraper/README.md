# TikTok Comments Scraper

Extract comments and optional nested replies from public TikTok video URLs through Scrappa. Use it for creator research, campaign monitoring, social listening, sentiment analysis, and comment export workflows.

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| url | string | Yes | Public TikTok video URL |
| count | integer | No | Number of comments to return, 1-50 |
| cursor | string | No | Pagination cursor from a previous response |
| includeReplies | boolean | No | Fetch replies for top-level comments with replies |
| maxRepliesPerComment | integer | No | Maximum replies to fetch per top-level comment, 1-500 |

## Example Input

    {
      "url": "https://www.tiktok.com/@tiktok/video/7568510388342443294",
      "count": 20,
      "includeReplies": true,
      "maxRepliesPerComment": 50
    }

## Output

Each saved comment is one default dataset item. Top-level comments have comment_type set to comment and null parent fields. Replies have comment_type set to reply with their parent comment ID and text.

The full top-level comments response is saved to the OUTPUT key-value record, including data.hasMore and data.cursor for the next comment page. When reply collection is enabled and replies are returned, raw reply responses are saved to REPLIES_OUTPUT, grouped by parent comment ID.

## Pagination and replies

Run once without cursor. If OUTPUT.data.hasMore is true, run again with cursor set to OUTPUT.data.cursor. This top-level pagination path is unchanged when includeReplies is enabled.

Reply collection is sequential. Scrappa requests time out after 60 seconds and are not retried. Apify storage requests retry network failures, HTTP 429, and HTTP 5xx responses up to eight times with exponential backoff starting at 500 milliseconds. The actor's five-minute run timeout remains in place.

## Pay-per-event budget

Before writing results, the actor reads the run's event prices, charged event counts, and spending limit. It writes only the prefix of dataset rows that fits the remaining budget. Apify's built-in apify-default-dataset-item event charges each row written to the default dataset. Large dataset writes are split into ordered batches under Apify's payload limit. The raw API responses remain available in the key-value outputs.

## Local development

Run the focused actor tests:

    cargo test --locked

Build the production image:

    docker build -f .actor/Dockerfile -t tiktok-comments-scraper .

The runtime uses SCRAPPA_API_KEY from Actor settings, the default Apify input key-value record and dataset, and APIFY_TOKEN for storage access. ACTOR_INPUT_KEY can override the input record key; APIFY_API_PUBLIC_BASE_URL and SCRAPPA_API_BASE_URL support local HTTP mocks and default to Apify and Scrappa production APIs.

## Support

For higher-volume usage or direct API access, use Scrappa at https://scrappa.co.

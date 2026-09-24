# TikTok Following Scraper

Extract the public TikTok accounts a creator follows through Scrappa. Use it for audience research, influencer discovery, creator vetting, social graph analysis, competitive monitoring, and following-list sampling workflows.

## Features

- Look up a TikTok username, full profile URL, or numeric user ID
- Fetch followed-account profile and verification metadata
- Paginate through Scrappa's time marker when more than 50 accounts are requested
- Write one dataset item per followed account
- Save the Scrappa response or a compact run summary to the OUTPUT key-value-store record
- Respect the Apify pay-per-event spending limit before writing dataset items

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| **profile** | string | Yes | TikTok username with or without @, full profile URL, or numeric user ID. Bare numeric values are user IDs; prefix numeric usernames with @. |
| **count** | integer | No | Maximum number of followed accounts to return. Any positive integer is accepted; requests above 50 use multiple Scrappa pages. |
| **time** | integer | No | Following pagination marker from a previous run. Leave empty for the first page. |

The actor also accepts **cursor** as an API input compatibility alias and sends it to Scrappa as **time**.

## Example Input

~~~json
{
  "profile": "@tiktok",
  "count": 10
}
~~~

## Output

Each followed account is saved as one dataset item:

~~~json
{
  "user_id": "107955",
  "unique_id": "tiktok",
  "nickname": "TikTok",
  "avatar": "https://example.com/avatar.jpeg",
  "follower_count": 162300000,
  "verified": true,
  "lookup_unique_id": "@tiktok",
  "lookup_user_id": null
}
~~~

For requests up to 50 accounts, OUTPUT contains the full last Scrappa API response. For larger requests, OUTPUT contains **following_extracted**, **requested_count**, **pages_fetched**, **has_next_page**, **next_time**, and **processed_time**.

## Local development

Run focused actor tests with **cargo test --locked**. Build the actor image from this directory with:

~~~sh
docker build -f .actor/Dockerfile -t tiktok-following-scraper .
~~~

The actor uses SCRAPPA_API_KEY for Scrappa authentication. SCRAPPA_API_BASE_URL and APIFY_API_PUBLIC_BASE_URL can override service endpoints for local smoke runs.

## Support

For higher-volume usage or direct API access, use Scrappa at https://scrappa.co.

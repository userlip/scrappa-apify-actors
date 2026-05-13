# TikTok Comments Scraper

Apify actor for Scrappa's `/api/tiktok/comments/list` and `/api/tiktok/comments/replies` endpoints.

`OUTPUT` always contains the top-level comments response for pagination. When reply collection is enabled, raw reply responses are saved separately to `REPLIES_OUTPUT`.

## Local Development

```bash
npm install
npm test
```

## Example Input

```json
{
  "url": "https://www.tiktok.com/@tiktok/video/7568510388342443294",
  "count": 20,
  "includeReplies": true,
  "maxRepliesPerComment": 50
}
```

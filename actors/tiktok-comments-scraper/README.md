# TikTok Comments Scraper

Apify actor for Scrappa's `/api/tiktok/comments/list` and `/api/tiktok/comments/replies` endpoints.

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

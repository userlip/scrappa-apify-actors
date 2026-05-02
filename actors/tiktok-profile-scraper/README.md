# TikTok Profile Scraper

Apify actor for Scrappa's `/api/tiktok/user/profile` endpoint.

## Local Development

```bash
npm install
npm test
```

## Example Input

```json
{
  "unique_id": "@tiktok"
}
```

You can also provide a full profile URL in `unique_id`, such as `https://www.tiktok.com/@tiktok`, or use `user_id` when you have the numeric TikTok user ID.

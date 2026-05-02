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
  "profile": "@tiktok"
}
```

You can also provide a full profile URL, such as `https://www.tiktok.com/@tiktok`, or a numeric TikTok user ID in `profile`. Bare numeric values are treated as user IDs; prefix numeric usernames with `@`.

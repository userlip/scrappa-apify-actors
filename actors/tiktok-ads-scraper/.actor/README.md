# TikTok Ads Scraper

Scrape TikTok Creative Center ad details from ad URLs with a lightweight Apify Actor powered by Scrappa.

This actor is built for TikTok ad research workflows: competitor creative monitoring, agency reporting, landing page checks, and collecting public ad metadata from TikTok Creative Center URLs.

## What You Get

- Ad ID and request URL
- Advertiser, brand, and account fields when returned by TikTok
- Creative text, CTA, objective, industry, category, region, country, and language metadata
- Landing page or destination URL
- Video, cover, and other media URLs
- Like, comment, and share counts when available
- Cached metadata when Scrappa returns cache information
- Raw nested Scrappa fields preserved in each dataset item

## Input

Paste one or more TikTok Creative Center ad URLs:

```json
{
  "urls": [
    "https://ads.tiktok.com/business/creativecenter/topads/7213160569871581185/pc/en?countryCode=US&period=30"
  ]
}
```

API callers can also use the legacy single URL field:

```json
{
  "url": "https://ads.tiktok.com/business/creativecenter/topads/7213160569871581185/pc/en?countryCode=US&period=30"
}
```

The actor pushes one dataset item per input URL. It does not crawl TikTok Creative Center searches or discover new ads; provide the ad detail URLs you want to resolve.

## Output

Example dataset item:

```json
{
  "ad_id": "7213160569871581185",
  "advertiser_name": "Example advertiser",
  "brand_name": "Example brand",
  "industry": "E-commerce",
  "objective": "Conversions",
  "creative_text": "Example ad text",
  "landing_page": "https://example.com",
  "video_url": "https://...",
  "cover": "https://...",
  "media_urls": ["https://..."],
  "like_count": 1200,
  "comment_count": 42,
  "share_count": 18,
  "cached": true,
  "request_url": "https://ads.tiktok.com/business/creativecenter/topads/7213160569871581185/pc/en",
  "request_ad_id": "7213160569871581185",
  "request_index": 1,
  "result_found": true
}
```

If TikTok rejects, expires, or region-locks an ad URL, the actor still writes a dataset row for that input with `result_found: false` and an `error_message`.

## Notes

This actor calls Scrappa's `/api/tiktok/ads/details` endpoint. The scraping work runs on Scrappa infrastructure; Apify is used for input validation, orchestration, and dataset output.

The Scrappa API docs also show the shorter Creative Center form `https://ads.tiktok.com/business/creativecenter/topads/7221117041168252930/`. TikTok currently serves many live ad detail pages with `/pc/en` appended, and this actor accepts both forms.

# Instagram Post Info Scraper

Extract public Instagram post details from a post URL or shortcode. This Actor is built for post research, creator and brand monitoring, engagement checks, campaign reporting, and lightweight enrichment workflows.

No Instagram login, cookies, proxy setup, or browser session is required. Provide one public Instagram post and the Actor saves the available post metadata to an Apify dataset.

## What You Can Scrape

- Post identifiers such as Instagram media ID, shortcode, permalink, and product type.
- Engagement metrics such as likes, comments, plays, and views when Instagram returns them.
- Caption text, hashtags, posted time, and accessibility caption.
- Media assets such as image variants, thumbnails, and video URLs.
- Author, collaborators, tagged users, location, and paid partnership flag when available.
- Raw response fields from the upstream post lookup, saved as one dataset item.

Private or removed posts may not return data. This Actor only reads public information available for the supplied post and does not bypass Instagram privacy restrictions.

## Input

Use one Instagram post per run. The recommended input is a full post URL:

```json
{
  "url": "https://www.instagram.com/natgeo/p/DXHKcyvEWfr/"
}
```

You can also provide the shortcode directly:

```json
{
  "shortcode": "DXHKcyvEWfr"
}
```

The legacy `media_id` field is still accepted for compatibility and is treated as a shortcode.

### Input Fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `url` | String | Yes, unless `shortcode` or `media_id` is provided | Full Instagram post URL. Recommended format: `https://www.instagram.com/username/p/SHORTCODE/`. |
| `shortcode` | String | Yes, unless `url` or `media_id` is provided | Instagram post shortcode, such as `DXHKcyvEWfr`. |
| `media_id` | String | No | Legacy input alias. Treated as the Instagram post shortcode for older integrations. |

## Tested Input

This README is based on a successful Apify run tested with:

```json
{
  "url": "https://www.instagram.com/natgeo/p/DXHKcyvEWfr/"
}
```

The run returned one dataset item for shortcode `DXHKcyvEWfr` with engagement metrics, caption, hashtags, media URLs, author data, collaborators, and tagged users.

## Output

Each run saves one result object to the default Apify dataset. The top-level response contains `success` and `data`. Most post fields are inside `data`.

Common output fields include:

| Field | Description |
| --- | --- |
| `success` | Whether the Scrappa request completed successfully. |
| `data.id` | Instagram media ID. |
| `data.shortcode` | Instagram post shortcode. |
| `data.media_type` | Media type, for example `image`, `video`, or carousel-related values. |
| `data.caption` | Public post caption text. |
| `data.hashtags` | Hashtags parsed from the caption when available. |
| `data.taken_at` | Post timestamp in ISO-like format when returned. |
| `data.taken_at_timestamp` | Post timestamp as a Unix timestamp when returned. |
| `data.like_count` | Number of likes when available. |
| `data.comment_count` | Number of comments when available. |
| `data.play_count` | Number of plays for video or Reels-style posts when available. |
| `data.view_count` | Number of views when available. |
| `data.media` | Media objects with images, thumbnail URL, video URL, or related media fields. |
| `data.author` | Post author metadata, commonly including `username`. |
| `data.collaborators` | Collaborating accounts when Instagram returns them. |
| `data.tagged_users` | Users tagged in the post when available. |
| `data.location` | Location data when attached to the post. |
| `data.is_paid_partnership` | Whether Instagram marks the post as a paid partnership. |
| `data.product_type` | Instagram product type such as `clips`. |
| `data.permalink` | Canonical Instagram post URL. |
| `data.accessibility_caption` | Accessibility caption when available. |

Example dataset item:

```json
{
  "success": true,
  "data": {
    "id": "3875111963462821867",
    "shortcode": "DXHKcyvEWfr",
    "media_type": "video",
    "caption": "One trip to Italy is never enough...",
    "hashtags": ["TucciinItaly", "TucciInItaly"],
    "taken_at": "2026-04-14T13:00:04+00:00",
    "taken_at_timestamp": 1776171604,
    "like_count": 94087,
    "comment_count": 1301,
    "play_count": 1631739,
    "view_count": 1631739,
    "media": [
      {
        "type": "video",
        "thumbnail_url": "https://...",
        "video_url": "https://..."
      }
    ],
    "author": {
      "username": "natgeo"
    },
    "collaborators": [
      { "username": "hulu", "full_name": null },
      { "username": "disneyplus", "full_name": null }
    ],
    "tagged_users": [
      { "username": "natgeotv", "full_name": "National Geographic TV" }
    ],
    "location": null,
    "is_paid_partnership": false,
    "product_type": "clips",
    "permalink": "https://www.instagram.com/natgeo/p/DXHKcyvEWfr/",
    "accessibility_caption": null
  }
}
```

The shortened `https://...` media values above stand in for real Instagram CDN URLs returned by the Actor. Exact fields can vary by post type and by what Instagram exposes publicly for that post.

## Exporting Results

Results are stored in the Actor's default dataset. From Apify, you can export the dataset as:

- JSON
- CSV
- Excel
- XML
- RSS
- HTML table

Use JSON for full nested media fields. Use CSV or Excel when you want spreadsheet-friendly post metrics for reporting, monitoring, or lead research.

## Pricing Position

This Actor is positioned for low-cost Instagram post enrichment at **$0.20 per 1,000 results**. It is a good fit when you want Apify-native runs, datasets, schedules, webhooks, and exports without building your own integration.

Because this Actor looks up one Instagram post per run, one result means one post dataset item.

For high-volume workflows, direct backend enrichment, or custom commercial plans, Scrappa direct API access is usually the better upgrade path.

## Upgrade to Scrappa Direct API

Need to look up many Instagram posts from your own app, data pipeline, CRM, or enrichment service? Use Scrappa directly instead of running one Apify task per lookup.

Scrappa direct API gives you:

- Direct HTTPS access to the same Instagram Post API used by this Actor.
- Lower-friction integration for backend jobs, queues, and batch processing.
- Centralized API-key based access for higher-volume Scrappa workflows.
- A simpler path when you need custom limits, combined endpoints, or broader Scrappa data products.

Start with this Actor when you want Apify storage, scheduling, and no-code exports. Move to Scrappa direct API when you need application-level integration or sustained volume.

## Typical Use Cases

- Check engagement metrics for a public Instagram post.
- Enrich campaign, creator, or influencer post URLs.
- Monitor brand, competitor, or publisher post performance.
- Export post captions, hashtags, media URLs, and tagged accounts.
- Validate post URLs before running larger Instagram data workflows.
- Feed public post metadata into BI, CRM, or reporting systems.

## Notes and Limits

- Run one Instagram post per Actor run.
- Use a full post URL for the most reliable input.
- The `shortcode` and legacy `media_id` fields are accepted for compatibility.
- The hosted Scrappa Actor already has `SCRAPPA_API_KEY` configured. If you fork or self-deploy this Actor, add `SCRAPPA_API_KEY` as an Actor environment variable before running it.
- Availability of likes, views, media URLs, collaborators, and tagged users depends on public data returned for the post.
- This Actor does not require or accept Instagram credentials.
- This Actor does not access private post content or bypass Instagram restrictions.

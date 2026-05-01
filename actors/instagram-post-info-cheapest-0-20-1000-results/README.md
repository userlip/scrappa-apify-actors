Retrieve detailed Instagram post data from a full Instagram post URL or shortcode. The dataset output exposes post fields at the top level for table export, including engagement metrics, media URLs, author details, captions, and post content. The full Scrappa API response is also stored in key-value storage as `OUTPUT`.

Recommended input:

```json
{
  "url": "https://www.instagram.com/natgeo/p/DXHKcyvEWfr/"
}
```

You can also provide a `shortcode` directly. The legacy `media_id` input is still accepted for compatibility and is treated as a shortcode.

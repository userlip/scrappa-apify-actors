Retrieve detailed Instagram post data from a full Instagram post URL or shortcode. Returns engagement metrics, media URLs, author details, captions, and post content in JSON format.

Recommended input:

```json
{
  "url": "https://www.instagram.com/instagram/p/Dc30nJeRKKz/"
}
```

You can also provide a `shortcode` directly. The legacy `media_id` input is still accepted for compatibility and is treated as a shortcode.

## Availability and retries

If the single-post lookup is temporarily unavailable, URLs containing the account username enable a fallback through Scrappa's recent user posts. Only a post with the exact requested shortcode is returned; unrelated posts and upstream errors are never published as results. Older posts outside the recent feed still depend on the single-post lookup. Available fields may differ between these upstream sources.

Each attempt has a shared 60-second deadline across both endpoints. Two retries wait 5 and 15 seconds, for a maximum request-and-wait budget of 200 seconds. The default run uses 128 MB and a 300-second timeout.

The input form and automated QA use the prefilled example URL. No URL default is injected into API inputs, so explicit `shortcode` and `media_id` requests keep their intended target. Refresh the prefill if the example leaves the account's recent feed while the single-post endpoint remains unavailable.

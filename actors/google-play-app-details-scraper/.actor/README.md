# Google Play App Details Scraper

Retrieve public Google Play app listings with developer information, ratings, review totals, downloads, categories, permissions, and media links. Look up several package names in one run.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `search_parameters` | Object | Google Play product, store, language, and country settings used for this lookup. |
| `app` | Object | Google Play identity and listing URL for the requested app. |
| `product_info` | Object | Listing facts such as title, developer, rating, review count, download band, and product link. |
| `media` | Object | Public app image and screenshot URLs included in the listing. |
| `about_this_app` | Object | App summary, update details, publisher information, and requested permissions. |
| `categories` | Array\<Object\> | Google Play categories associated with the app listing. |
| `updated_on` | String | Updated On value associated with this google play app details record. |
| `ratings` | Array\<Object\> | Rating-count breakdown grouped by star value. |
| `input_product_id` | String | Apple or Google Play product identifier supplied for this lookup. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- App teams can compare listing details and public ratings across competitors.
- Mobile analysts can monitor download bands, categories, and update information.
- Product researchers can collect app metadata for a storefront catalog.

## How to use

1. Add one app package name to each entry in `apps`.
2. Choose the Google Play country and language for the storefront response.
3. Review the product details, ratings, and permission summary in the dataset.

```json
{
  "apps": [
    {
      "product_id": "com.spotify.music"
    }
  ],
  "hl": "en",
  "gl": "us",
  "store": "apps",
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `apps` | Array\<object\> | Yes | Google Play product IDs or app package names to look up. |
| `apps[].product_id` | string | Yes per entry | Google Play product id. For apps this is the package name. |
| `apps[].id` | string | No | Legacy alias for product\_id. |
| `apps[].url` | string | No | Public source page URL used to retrieve this record. |
| `apps[].hl` | string | No | Interface language code \(e.g. en, de\). |
| `apps[].gl` | string | No | Country code for the storefront \(2 letters, e.g. us, de\). |
| `apps[].store` | string | No | Google Play store vertical: apps, movies, tv, books, or audiobooks. Accepted values: apps, movies, tv, books, audiobooks. |
| `apps[].season_id` | string | No | TV season id for Google Play TV products. |
| `apps[].all_reviews` | boolean | No | Request Google Play review-expanded product pages when available. |
| `apps[].platform` | string | No | Review platform filter, such as phone, tablet, tv, chromebook, watch, or car. Accepted values: phone, tablet, tv, chromebook, watch, car. |
| `apps[].rating` | string | No | Review rating filter from 1 to 5. Accepted values: 1, 2, 3, 4, 5. |
| `apps[].sort_by` | string | No | Review sort mode, such as 1, 2, 3, most\_relevant, newest, or rating. Accepted values: 1, 2, 3, most\_relevant, newest, rating. |
| `apps[].num` | integer | No | Requested review result count for clients that need SerpApi-compatible parameters. |
| `apps[].next_page_token` | string | No | Review pagination token for clients that need SerpApi-compatible parameters. |
| `id` | string | No | Legacy alias for product\_id. |
| `url` | string | No | Public source page URL used to retrieve this record. |
| `hl` | string | No | Interface language code \(e.g. en, de\). |
| `gl` | string | No | Country code for the storefront \(2 letters, e.g. us, de\). |
| `store` | string | No | Google Play store vertical: apps, movies, tv, books, or audiobooks. Accepted values: apps, movies, tv, books, audiobooks. |
| `season_id` | string | No | TV season id for Google Play TV products. |
| `all_reviews` | boolean | No | Request Google Play review-expanded product pages when available. |
| `platform` | string | No | Review platform filter, such as phone, tablet, tv, chromebook, watch, or car. Accepted values: phone, tablet, tv, chromebook, watch, car. |
| `rating` | string | No | Review rating filter from 1 to 5. Accepted values: 1, 2, 3, 4, 5. |
| `sort_by` | string | No | Review sort mode, such as 1, 2, 3, most\_relevant, newest, or rating. Accepted values: 1, 2, 3, most\_relevant, newest, rating. |
| `num` | integer | No | Requested review result count for clients that need SerpApi-compatible parameters. |
| `next_page_token` | string | No | Review pagination token for clients that need SerpApi-compatible parameters. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "search_parameters": {
    "engine": "google_play",
    "product_id": "com.northstar.trailnotes",
    "id": "com.northstar.trailnotes",
    "store": "apps",
    "hl": "en",
    "gl": "us"
  },
  "app": {
    "app_id": "com.northstar.trailnotes",
    "title": "Trail Notes: Hiking Planner",
    "developer": "Northstar Mobile, Inc.",
    "url": "https://play.google.com/store/apps/details?id=com.northstar.trailnotes"
  },
  "product_info": {
    "title": "Trail Notes: Hiking Planner",
    "authors": [
      {
        "name": "Northstar Mobile, Inc.",
        "link": "https://play.google.com/store/apps/developer?id=Northstar+Mobile"
      }
    ],
    "rating": 4.7,
    "reviews": 18420,
    "downloads": "1M+",
    "thumbnail": "https://play-lh.googleusercontent.com/trail-notes-icon",
    "product_id": "com.northstar.trailnotes",
    "link": "https://play.google.com/store/apps/details?id=com.northstar.trailnotes"
  },
  "media": {
    "images": [
      "https://play-lh.googleusercontent.com/trail-notes-screen-1"
    ]
  },
  "about_this_app": {
    "snippet": "Offline trail maps, route planning, and field notes for day hikes.",
    "updated_on": "Sep 18, 2026",
    "downloads": "1M+",
    "offered_by": "Northstar Mobile, Inc.",
    "permissions": [
      {
        "type": "Location",
        "details": [
          "Approximate location for nearby trail suggestions"
        ]
      }
    ]
  },
  "categories": [
    {
      "name": "Maps & Navigation",
      "link": "https://play.google.com/store/apps/category/MAPS_AND_NAVIGATION",
      "category_id": "MAPS_AND_NAVIGATION"
    }
  ],
  "updated_on": "2026-09-18",
  "ratings": [
    {
      "stars": 5,
      "count": 12400
    },
    {
      "stars": 4,
      "count": 3900
    }
  ],
  "input_product_id": "com.spotify.music",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.50 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Set **maxResults** to cap the number of dataset items saved in one run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~google-play-app-details-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Where do I find an app package name?

Copy the value after `id=` in the Google Play listing URL, such as `com.spotify.music`.

## Related Scrappa Actors

- [Apple App Store App Details Scraper](https://apify.com/thescrappa/apple-app-store-app-details-scraper)
- [Apple App Store Search Scraper](https://apify.com/thescrappa/apple-app-store-search-scraper)
- [Apple App Store Reviews Scraper](https://apify.com/thescrappa/apple-app-store-reviews-scraper)

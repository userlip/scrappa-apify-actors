# Apple App Store Top Charts Scraper

Retrieve public Apple App Store chart entries with rank, app name, developer, product ID, genre, and listing URL. Compare chart types across storefront countries in one run.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `position` | Integer | Rank of the app in the selected Apple storefront chart. |
| `id` | String | Apple product identifier for the ranked app. |
| `title` | String | Title displayed by the source for this record. |
| `artist` | String | Artist value associated with this apple app store top charts record. |
| `url` | String | Public source or listing URL associated with this record. |
| `artwork` | String | Artwork value associated with this apple app store top charts record. |
| `genres` | Array\<String\> | Apple genre labels associated with the app. |
| `input_chart` | String | Apple chart type requested for this storefront. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- App publishers can monitor rankings for free, paid, or grossing charts.
- Mobile analysts can compare app visibility between storefront countries.
- Investors can track public chart movement across competing app categories.

## How to use

1. Add one chart type per entry in `charts`.
2. Select the Apple store vertical and storefront country.
3. Use the result limit to focus on the ranks you need.

```json
{
  "charts": [
    {
      "chart": "top-free"
    }
  ],
  "store": "apps",
  "country": "us",
  "limit": 50,
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `charts` | Array\<object\> | Yes | Apple chart types to retrieve for the selected storefront and store. |
| `charts[].chart` | string | Yes per entry | Chart type: top-free, top-paid, top-grossing, most-played, top-subscriber, or top-episodes. Music serves most-played albums, podcasts serve top shows; the response echoes requested and served chart types Accepted values: top-free, top-paid, top-grossing, most-played, top-subscriber, top-episodes. |
| `charts[].store` | string | No | Store vertical: apps, mac, books, audiobooks, movies, tv, music, or podcasts Accepted values: apps, mac, books, audiobooks, movies, tv, music, podcasts. |
| `charts[].genre` | string | No | Apple genre id, passed through to the charts feeds as-is |
| `charts[].country` | string | No | Kununu country market code, such as de, at, or ch. |
| `charts[].limit` | integer | No | Maximum number of company matches requested for this search page. |
| `store` | string | No | Store vertical: apps, mac, books, audiobooks, movies, tv, music, or podcasts Accepted values: apps, mac, books, audiobooks, movies, tv, music, podcasts. |
| `genre` | string | No | Apple genre id, passed through to the charts feeds as-is |
| `country` | string | No | Kununu country market code, such as de, at, or ch. |
| `limit` | integer | No | Maximum number of company matches requested for this search page. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "position": 1,
  "id": "com.northstar.trailnotes",
  "title": "Trail Notes: Hiking Planner",
  "artist": "Northstar Mobile, Inc.",
  "url": "https://apps.apple.com/us/app/trail-notes-hiking-planner/id1234567890",
  "artwork": "https://is1-ssl.mzstatic.com/image/thumb/app-icon/512x512bb.jpg",
  "genres": [
    "Navigation"
  ],
  "input_chart": "top-free",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Set **maxResults** to cap the number of dataset items saved in one run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~apple-app-store-charts-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Which chart types are available?

Values include top-free, top-paid, top-grossing, most-played, top-subscriber, and top-episodes. Availability depends on the selected store.

## Related Scrappa Actors

- [Apple App Store Search Scraper](https://apify.com/thescrappa/apple-app-store-search-scraper)
- [Apple App Store App Details Scraper](https://apify.com/thescrappa/apple-app-store-app-details-scraper)
- [Apple App Store Developer Apps Scraper](https://apify.com/thescrappa/apple-app-store-developer-scraper)

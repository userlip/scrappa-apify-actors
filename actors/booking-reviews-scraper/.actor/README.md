# Booking.com Reviews Scraper

Collect public Booking.com guest reviews with ratings, review text, reviewer names, stay dates, and language labels. Add multiple hotel URLs and page through available reviews.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `review_id` | String | Review Id value associated with this booking.com reviews record. |
| `rating` | String | Rating value reported by the source. |
| `title` | String | Title displayed by the source for this record. |
| `text` | String | Text value associated with this booking.com reviews record. |
| `positive_text` | String | Positive Text value associated with this booking.com reviews record. |
| `negative_text` | String | Negative Text value associated with this booking.com reviews record. |
| `author` | String | Public display name supplied by the source. |
| `traveler_type` | String | Traveler Type value associated with this booking.com reviews record. |
| `stay_date` | String | Stay Date value associated with this booking.com reviews record. |
| `created_at` | String | Created At value associated with this booking.com reviews record. |
| `language` | String | Language value associated with this booking.com reviews record. |
| `input_url` | String | Hotel or job page URL supplied for this lookup. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Hotel analysts can compare guest feedback and scores across properties.
- Travel businesses can monitor public review themes for destination research.
- Hospitality teams can track recent feedback and identify service patterns.

## How to use

1. Add a Booking.com hotel URL to each entry in `hotels`.
2. Choose review language, currency, sort order, and page size if needed.
3. Set `maxPages` and `maxResults` to cap the review collection.

```json
{
  "hotels": [
    {
      "url": "https://www.booking.com/hotel/de/adlon-kempinski-berlin.html"
    }
  ],
  "lang": "en-us",
  "currency": "EUR",
  "limit": 10,
  "maxResults": 10,
  "maxPages": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `hotels` | Array\<object\> | Yes | Hotel listing URLs whose public guest reviews you want to collect. |
| `hotels[].url` | string | Yes per entry | Public source page URL used to retrieve this record. |
| `hotels[].country` | string | No | Kununu country market code, such as de, at, or ch. |
| `hotels[].slug` | string | No | Hotel slug from the Booking.com URL \(e.g. ritz-paris\). The trailing .html is optional. |
| `hotels[].lang` | string | No | Review language hint \(e.g. en-us, de\). |
| `hotels[].currency` | string | No | 3-letter currency code \(e.g. EUR, USD\). |
| `hotels[].page` | integer | No | One-based result page to request. |
| `hotels[].limit` | integer | No | Maximum number of company matches requested for this search page. |
| `hotels[].sort` | string | No | Sort order for returned search results. Accepted values: recent\_desc, recent\_asc, score\_desc, score\_asc. |
| `country` | string | No | Kununu country market code, such as de, at, or ch. |
| `slug` | string | No | Hotel slug from the Booking.com URL \(e.g. ritz-paris\). The trailing .html is optional. |
| `lang` | string | No | Review language hint \(e.g. en-us, de\). |
| `currency` | string | No | 3-letter currency code \(e.g. EUR, USD\). |
| `page` | integer | No | One-based result page to request. |
| `limit` | integer | No | Maximum number of company matches requested for this search page. |
| `sort` | string | No | Sort order for returned search results. Accepted values: recent\_desc, recent\_asc, score\_desc, score\_asc. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "review_id": "review-884210",
  "rating": "9.2",
  "title": "A comfortable city stay",
  "text": "The room was quiet, the staff were attentive, and the location made museums easy to reach.",
  "positive_text": "Helpful staff and a calm room.",
  "negative_text": "Breakfast service was busy before 9 am.",
  "author": "Jordan Lee",
  "traveler_type": "Couple",
  "stay_date": "September 2026",
  "created_at": "2026-09-18",
  "language": "en-us",
  "input_url": "https://www.booking.com/hotel/de/adlon-kempinski-berlin.html",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Pagination follows the source response. Set **maxPages** per batch entry and **maxResults** across the run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items and **maxPages** to limit pages for each entry. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~booking-reviews-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can I use a hotel slug instead of a full URL?

This Actor accepts hotel URLs in its batch. If you only have a slug and country, open the property page to copy its canonical URL first.

## Related Scrappa Actors

- [Booking.com Search Scraper](https://apify.com/thescrappa/booking-search-scraper)
- [Booking.com Hotel Details Scraper](https://apify.com/thescrappa/booking-hotel-details-scraper)
- [Booking.com Review Scores Scraper](https://apify.com/thescrappa/booking-review-scores-scraper)

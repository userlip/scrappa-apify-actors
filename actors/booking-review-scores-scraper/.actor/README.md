# Booking.com Review Scores Scraper

Retrieve aggregate Booking.com hotel ratings and review totals without collecting full review text. Batch property URLs to compare guest scores across hotels.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `success` | Boolean | True when Booking.com returned a parsed review-score response. |
| `data` | Object | Property identity and aggregate rating values, including score scale and review count. |
| `meta` | Object | Fetch metadata including property URL, timing, cache state, and collection steps. |
| `input_url` | String | Hotel or job page URL supplied for this lookup. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Hotel analysts can benchmark aggregate guest scores across properties.
- Travel sites can enrich hotel records with public rating totals.
- Investors can track guest-rating trends for lodging portfolios.

## How to use

1. Add one Booking.com hotel URL per entry in `hotels`.
2. Choose language and currency hints for the property page.
3. Read the aggregate rating and review count from the returned data object.

```json
{
  "hotels": [
    {
      "url": "https://www.booking.com/hotel/de/adlon-kempinski-berlin.html"
    }
  ],
  "lang": "en-us",
  "currency": "EUR",
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `hotels` | Array\<object\> | Yes | Hotel listing URLs whose aggregate guest rating and review totals you want to retrieve. |
| `hotels[].url` | string | Yes per entry | Public source page URL used to retrieve this record. |
| `hotels[].country` | string | No | Kununu country market code, such as de, at, or ch. |
| `hotels[].slug` | string | No | Hotel slug from the Booking.com URL \(e.g. ritz-paris\). The trailing .html is optional. |
| `hotels[].checkin` | string | No | Hotel check-in date in YYYY-MM-DD format. |
| `hotels[].checkout` | string | No | Hotel check-out date in YYYY-MM-DD format, after check-in. |
| `hotels[].group_adults` | integer | No | Number of adults \(1-30\). |
| `hotels[].group_children` | integer | No | Number of children \(0-20\). |
| `hotels[].no_rooms` | integer | No | Number of rooms \(1-30\). |
| `hotels[].lang` | string | No | Language code for the response \(e.g. en-us, de\). |
| `hotels[].currency` | string | No | Three-letter currency code \(e.g. EUR, USD\). |
| `country` | string | No | Kununu country market code, such as de, at, or ch. |
| `slug` | string | No | Hotel slug from the Booking.com URL \(e.g. ritz-paris\). The trailing .html is optional. |
| `checkin` | string | No | Hotel check-in date in YYYY-MM-DD format. |
| `checkout` | string | No | Hotel check-out date in YYYY-MM-DD format, after check-in. |
| `group_adults` | integer | No | Number of adults \(1-30\). |
| `group_children` | integer | No | Number of children \(0-20\). |
| `no_rooms` | integer | No | Number of rooms \(1-30\). |
| `lang` | string | No | Language code for the response \(e.g. en-us, de\). |
| `currency` | string | No | Three-letter currency code \(e.g. EUR, USD\). |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "success": true,
  "data": {
    "title": "Adlon Kempinski Berlin",
    "canonical_url": "https://www.booking.com/hotel/de/adlon-kempinski-berlin.html",
    "aggregate_rating": {
      "reviewCount": 286,
      "bestRating": 10,
      "@type": "AggregateRating",
      "ratingValue": 9.2
    },
    "review_score": {
      "rating_value": 9.2,
      "best_rating": 10,
      "review_count": 286
    },
    "parsed": true
  },
  "meta": {
    "url": "https://www.booking.com/hotel/de/adlon-kempinski-berlin.html",
    "duration_ms": 495,
    "scraped_at": "2026-09-28T14:22:00Z",
    "cached": true,
    "booking_pool_attempts": 1,
    "booking_pool_stages": [
      "review-summary"
    ]
  },
  "input_url": "https://www.booking.com/hotel/de/adlon-kempinski-berlin.html",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~booking-review-scores-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### How does this differ from the reviews Actor?

This Actor returns aggregate scores and review counts. Use Booking.com Reviews Scraper for individual review records and comments.

## Related Scrappa Actors

- [Booking.com Reviews Scraper](https://apify.com/thescrappa/booking-reviews-scraper)
- [Booking.com Hotel Details Scraper](https://apify.com/thescrappa/booking-hotel-details-scraper)
- [Booking.com Search Scraper](https://apify.com/thescrappa/booking-search-scraper)

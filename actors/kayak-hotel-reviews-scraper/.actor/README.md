# Kayak Hotel Reviews Scraper

Collect Kayak hotel review summaries with scores, rating labels, positive and negative comments, source sites, and date labels. Batch hotel IDs to compare guest feedback.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `id` | String | Stable source identifier for this record. |
| `localizedMonthYear` | String | Month and year label shown for the review. |
| `score` | Number | Numeric guest score reported by the linked review source. |
| `localizedRatingCategory` | String | Localized rating category associated with the score. |
| `positiveComment` | String | Positive review excerpt returned for the property. |
| `negativeComment` | String | Critical review excerpt returned for the property. |
| `author` | String | Reviewer display name supplied by the source site. |
| `siteLink` | String | Link to the review source page. |
| `siteName` | String | Name of the site that supplied the review. |
| `siteLogo` | String | SiteLogo value associated with this kayak hotel reviews record. |
| `internal` | Boolean | Internal value associated with this kayak hotel reviews record. |
| `localizedScore` | String | LocalizedScore value associated with this kayak hotel reviews record. |
| `input_hotel_id` | Integer | KAYAK hotel identifier submitted for this lookup. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Hotel analysts can compare public review scores across competing properties.
- Travel researchers can track review themes and source coverage by hotel.
- Hospitality teams can monitor feedback patterns for selected destinations.

## How to use

1. Add a KAYAK hotel ID to each entry in `hotels`.
2. Choose a review sort order and requested amount.
3. Set `maxResults` to cap the saved review rows.

```json
{
  "hotels": [
    {
      "hotel_id": 15297
    }
  ],
  "sort": "recent",
  "amount": 10,
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `hotels` | Array\<object\> | Yes | KAYAK hotel identifiers whose public guest reviews you want to collect. |
| `hotels[].hotel_id` | integer | Yes per entry | KAYAK hotel identifier used to retrieve property rates or reviews. |
| `hotels[].sort` | string | No | Sort order for returned search results. Accepted values: recent, highest, lowest. |
| `hotels[].amount` | integer | No | Qualified KAYAK request parameter. |
| `sort` | string | No | Sort order for returned search results. Accepted values: recent, highest, lowest. |
| `amount` | integer | No | Qualified KAYAK request parameter. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "id": "kayak-review-22091",
  "localizedMonthYear": "September 2026",
  "score": 4.5,
  "localizedRatingCategory": "Excellent",
  "positiveComment": "Comfortable room and convenient access to the city.",
  "negativeComment": "The lobby was crowded in the afternoon.",
  "author": "Casey Morgan",
  "siteLink": "https://www.tripadvisor.com/Hotel_Review-Riverview_Seattle",
  "siteName": "Tripadvisor",
  "siteLogo": "https://kayak.com/images/tripadvisor.svg",
  "internal": false,
  "localizedScore": "4.5/5",
  "input_hotel_id": 15297,
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~kayak-hotel-reviews-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Does this return full reviews from every source?

KAYAK may provide review excerpts and source links. Available text and fields depend on each source response.

## Related Scrappa Actors

- [Kayak Hotels Search Scraper](https://apify.com/thescrappa/kayak-hotels-search-scraper)
- [Kayak Hotel Details Scraper](https://apify.com/thescrappa/kayak-hotel-details-scraper)
- [Booking.com Reviews Scraper](https://apify.com/thescrappa/booking-reviews-scraper)

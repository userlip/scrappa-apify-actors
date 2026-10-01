# Google Maps Reviews Scraper

Read Google Maps reviews with star ratings, written feedback, posting dates and owner replies. Start with a place ID and choose a review order or text filter for the public feedback.

## What data can you extract?

Place and route details follow the public Google Maps page; optional ratings, links and photos may not be shown for every record.

| Field | Type | Description |
| --- | --- | --- |
| `author_name` | text | Reviewer name shown for the place review by Google Maps, in the format used by the source; null when it is omitted. |
| `rating` | number | Star rating shown for this Google Maps review, on the source’s 1-to-5 scale; null if no rating is shown. |
| `review_text` | text | Text written in the public Google Maps review; null when the reviewer left no written text. |
| `timestamp` | number | Publication timestamp for this place review, as a timestamp in the source response format; null if Google Maps does not supply it. |
| `review_likes` | number | Review likes shown for the place review by Google Maps, in the format used by the source; null when it is omitted. |
| `review_language` | text | Review language shown for the place review by Google Maps, in the format used by the source; null when it is omitted. |
| `review_id` | text | review ID for the place review, assigned by Google Maps; null when the source does not expose it. |

## Use cases

- Customer experience teams can group public feedback by rating and date to spot recurring service issues.
- Local businesses can monitor reviewer comments and owner replies across locations.
- Researchers can compare review themes before adding places to a local directory.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Enter a company, shop, place or provider identifier and use the available sort, rating and page fields to focus the reviews.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "business_id": "0x808fba02425dad8f:0x6c296c66619367e0",
  "sort": 2,
  "limit": 10
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `business_id` | string | Yes | Google Maps business/place ID in 0x[hex]:0x[hex] format. To get one, run Google Maps Search or Advanced Search and copy the business_id field from the business you want to monitor. |
| `sort` | integer | Yes | Review order. Use Newest for monitoring, Most Relevant for representative samples, or rating sorts for sentiment QA. Constraints: minimum 1; maximum 4. |
| `limit` | integer | No | Number of reviews to return on this page. Start with 5-10 for a first test, then increase to 20 for production pagination. Constraints: minimum 1; maximum 20. |
| `page` | string | No | Paste the nextPage token from a previous run to retrieve the next page of reviews. |
| `search` | string | No | Optional keyword to match inside reviews, for example service, price, delivery, warranty, or rude. |
| `debug` | boolean | No | Enable debug output for troubleshooting |
| `use_cache` | boolean | No | Use cached results if available to reduce costs and speed up results |
| `maximum_cache_age` | integer | No | Maximum age of cached results in seconds. Set to 0 to always fetch fresh data. Constraints: minimum 0. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "author_name": "Morgan Lee",
  "rating": 4.7,
  "review_text": "Helpful staff answered my question clearly and followed up the same day.",
  "timestamp": 1790327700,
  "review_likes": 7.3,
  "review_language": "en",
  "review_id": "review_demo_184"
}
```

## Pricing

**Current live price:** $0.30 per 1,000 results.

Each saved review counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-maps-reviews-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Where do I find the Google Maps place ID?

Run Google Maps Search or Advanced Search and copy the business_id for the place. A supported Maps URL also works.

## Related Scrappa Actors

- [Google Maps Advanced Search Scraper](https://apify.com/thescrappa/google-maps-advanced-search-scraper)
- [Google Maps Autocomplete Scraper](https://apify.com/thescrappa/google-maps-autocomplete-scraper)
- [Google Maps Business Details Scraper](https://apify.com/thescrappa/google-maps-business-details-scraper)
- [Google Maps Directions Scraper](https://apify.com/thescrappa/google-maps-directions-scraper)
- [Google Maps Photos Scraper](https://apify.com/thescrappa/google-maps-photos-scraper)

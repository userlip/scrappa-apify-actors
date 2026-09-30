# Booking.com Search Scraper for Travel Planning

The Booking.com Search Scraper for Travel Planning collects hotel listings, availability, and property details from Booking.com. Provide a Booking.com destination; the actor saves source fields such as `name`, `url`, `image`, and `review_score` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Booking.com. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `name` | text | Name returned for this result. |
| `url` | link | URL returned for this result. |
| `image` | image | Image returned for this result. |
| `review_score` | number | Review Score returned for this result. |
| `review_score_word` | text | Review Label returned for this result. |
| `review_count` | number | Review Count returned for this result. |
| `location` | text | Location returned for this result. |
| `price` | text | Price returned for this result. |
| `currency` | text | Currency returned for this result. |
| `request_search_index` | number | Search Index returned for this result. |
| `request_ss` | text | Request Destination returned for this result. |
| `request_checkin` | date | Check-in returned for this result. |
| `request_checkout` | date | Check-out returned for this result. |
| `request_group_adults` | number | Adults returned for this result. |
| `request_group_children` | number | Children returned for this result. |
| `request_no_rooms` | number | Rooms returned for this result. |
| `request_lang` | text | Language returned for this result. |
| `request_currency` | text | Request Currency returned for this result. |

## Use cases

- Collect hotel listings, availability, and property details for a destination, route, or travel date.
- Compare returned options and details before planning a trip.
- Export travel records to a spreadsheet or booking research workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `searches` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "ss": "Paris",
  "searches": [
    {
      "ss": "Paris"
    }
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "name": "Example value",
  "url": "https://example.com/result/1",
  "image": "https://example.com/image.jpg",
  "review_score": 4.7,
  "review_score_word": "4.7",
  "review_count": 42,
  "location": "Example location"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `ss` | string | No | Booking.com destination search, such as Paris, New York, or Berlin. Required for single-search runs; use Batch Searches instead for multi-search runs. |
| `checkin` | string | No | Check-in date in YYYY-MM-DD format. Provide with Check-out Date for property cards. |
| `checkout` | string | No | Check-out date in YYYY-MM-DD format. Must be after Check-in Date. |
| `group_adults` | integer | No | Number of adults, from 1 to 30. Constraints: minimum 1; maximum 30. |
| `group_children` | integer | No | Number of children, from 0 to 20. Constraints: minimum 0; maximum 20. |
| `no_rooms` | integer | No | Number of rooms, from 1 to 30. Constraints: minimum 1; maximum 30. |
| `lang` | string | No | Booking.com UI language hint, such as en-us, en, de, or fr. |
| `currency` | string | No | Three-letter currency code such as USD, EUR, or GBP. |
| `searches` | array of object | No | Optional list of Booking.com searches to run in one actor run. When provided, these searches are used instead of the single-search fields above. Constraints: maximum 25 items. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Booking.com. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~booking-search-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Booking.com Hotel Details Scraper for Travel](https://apify.com/thescrappa/booking-hotel-details-scraper)
- [Google Flights Search Scraper for Travel Planning](https://apify.com/thescrappa/google-flights-search-scraper)
- [Google Hotels Autocomplete Scraper for Stays](https://apify.com/thescrappa/google-hotels-autocomplete-scraper)
- [Google Hotels Search Scraper for Travel Planning](https://apify.com/thescrappa/google-hotels-search-scraper)
- [Google Maps Directions Scraper for Travel Planning](https://apify.com/thescrappa/google-maps-directions-scraper)

# Google Hotels Autocomplete Scraper for Stays

The Google Hotels Autocomplete Scraper for Stays collects hotel and destination suggestions from Google Hotels. Provide a search phrase or a short list of phrases; the actor saves source fields such as `position`, `value`, `autocomplete_suggestion`, and `type` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Google Hotels. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | # returned for this result. |
| `value` | text | Suggestion returned for this result. |
| `autocomplete_suggestion` | text | Normalized Suggestion returned for this result. |
| `type` | text | Type returned for this result. |
| `property_token` | text | Property Token returned for this result. |
| `thumbnail` | image | Thumbnail returned for this result. |
| `scrappa_google_hotels_link` | link | Hotel Search returned for this result. |
| `source_query` | text | Source Query returned for this result. |
| `request_gl` | text | Country returned for this result. |
| `request_hl` | text | Language returned for this result. |
| `request_currency` | text | Currency returned for this result. |
| `request_type` | text | Requested Type returned for this result. |
| `response_time_ms` | number | Response Time returned for this result. |

## Use cases

- Collect hotel and destination suggestions for a destination, route, or travel date.
- Compare returned options and details before planning a trip.
- Export travel records to a spreadsheet or booking research workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `queries` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "queries": [
    "Berlin"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "position": 42,
  "value": "Example value",
  "autocomplete_suggestion": "Example value",
  "type": "Example value",
  "property_token": "Example value",
  "thumbnail": "https://example.com/image.jpg",
  "scrappa_google_hotels_link": "https://example.com/result/1"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | array of string | No | Destination, landmark, area, or hotel-name prefixes. You can also provide a comma-separated string through the API. Constraints: minimum 1 items; maximum 100 items. |
| `q` | string | No | Compatibility alias for one query. Queries and q are combined and deduplicated when both are provided. |
| `gl` | string | No | Two-letter Google country code, such as de, us, gb, or fr. |
| `hl` | string | No | Two-letter language code, such as en, de, es, or fr. |
| `currency` | string | No | Three-letter currency code used in generated hotel-search links. |
| `type` | string | No | Return locations, hotels/accommodations, or both. Constraints: allowed values: location, hotel, all. |

## Pricing

**Current live price:** $0.25 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Google Hotels. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-hotels-autocomplete-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Booking.com Hotel Details Scraper for Travel](https://apify.com/thescrappa/booking-hotel-details-scraper)
- [Booking.com Search Scraper for Travel Planning](https://apify.com/thescrappa/booking-search-scraper)
- [Google Flights Search Scraper for Travel Planning](https://apify.com/thescrappa/google-flights-search-scraper)
- [Google Hotels Search Scraper for Travel Planning](https://apify.com/thescrappa/google-hotels-search-scraper)
- [Google Maps Directions Scraper for Travel Planning](https://apify.com/thescrappa/google-maps-directions-scraper)

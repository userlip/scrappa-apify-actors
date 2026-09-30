# Google Hotels Search Scraper for Travel Planning

The Google Hotels Search Scraper for Travel Planning collects hotel listings, availability, and property details from Google Hotels. Provide a search phrase or a short list of phrases; the actor saves source fields such as `name`, `type`, `hotel_class`, and `overall_rating` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Google Hotels. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `name` | text | Name returned for this result. |
| `type` | text | Type returned for this result. |
| `hotel_class` | text | Class returned for this result. |
| `overall_rating` | number | Rating returned for this result. |
| `reviews` | number | Reviews returned for this result. |
| `rate_per_night_lowest` | text | Nightly Rate returned for this result. |
| `rate_per_night_extracted_lowest` | number | Nightly Rate Numeric returned for this result. |
| `total_rate_lowest` | text | Total Rate returned for this result. |
| `total_rate_extracted_lowest` | number | Total Rate Numeric returned for this result. |
| `booking_link` | link | Booking Link returned for this result. |
| `property_token` | text | Property Token returned for this result. |
| `entity_id` | text | Entity ID returned for this result. |
| `place_id` | text | Place ID returned for this result. |
| `latitude` | number | Latitude returned for this result. |
| `longitude` | number | Longitude returned for this result. |
| `thumbnail` | image | Thumbnail returned for this result. |
| `price_sources_count` | number | Price Sources returned for this result. |
| `amenities_count` | number | Amenities returned for this result. |
| `request_q` | text | Request Query returned for this result. |
| `request_check_in_date` | date | Check-in returned for this result. |
| `request_check_out_date` | date | Check-out returned for this result. |
| `request_adults` | number | Adults returned for this result. |
| `request_children` | number | Children returned for this result. |
| `request_currency` | text | Currency returned for this result. |
| `request_gl` | text | Country returned for this result. |
| `request_hl` | text | Language returned for this result. |

## Use cases

- Collect hotel listings, availability, and property details for a destination, route, or travel date.
- Compare returned options and details before planning a trip.
- Export travel records to a spreadsheet or booking research workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. Set the lookup fields to match the query or identifier you want to collect.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "q": "Paris, France",
  "check_in_date": "tomorrow",
  "check_out_date": "day-after-tomorrow"
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "name": "Example value",
  "type": "Example value",
  "hotel_class": "Example value",
  "overall_rating": 4.7,
  "reviews": 42,
  "rate_per_night_lowest": "Example value",
  "rate_per_night_extracted_lowest": 42
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `q` | string | Yes | Location, landmark, area, or hotel query to search in Google Hotels. |
| `check_in_date` | string | Yes | Check-in date in YYYY-MM-DD format, or use today, tomorrow, or day-after-tomorrow. Must resolve to today or a future date. |
| `check_out_date` | string | Yes | Check-out date in YYYY-MM-DD format, or use tomorrow or day-after-tomorrow. Must resolve after the check-in date. |
| `adults` | integer | No | Number of adults. Constraints: minimum 1; maximum 10. |
| `children` | integer | No | Number of children. If greater than 0, provide matching Children Ages. Constraints: minimum 0; maximum 6. |
| `children_ages` | array of integer | No | Ages for each child, 1-17. The number of ages must match Children. |
| `currency` | string | No | Three-letter currency code such as USD, EUR, or GBP. |
| `gl` | string | No | Two-letter Google country code such as us, fr, gb, or de. |
| `hl` | string | No | Two-letter language code such as en, fr, de, or es. |
| `sort_by` | string | No | Sort hotel results by lowest price, highest rating, or most reviewed. Constraints: allowed values: 3, 8, 13. |
| `min_price` | integer | No | Minimum nightly price. Constraints: minimum 0. |
| `max_price` | integer | No | Maximum nightly price. Must be greater than Minimum Price and within 5000. Constraints: minimum 1; maximum 5000. |
| `hotel_class` | string | No | Filter results by 2-star, 3-star, 4-star, or 5-star hotel class. Constraints: allowed values: 2, 3, 4, 5. |
| `rating` | string | No | Filter results by a minimum guest rating of 3.5+, 4.0+, or 4.5+. Constraints: allowed values: 7, 8, 9. |
| `free_cancellation` | boolean | No | Return only properties with free cancellation. Cannot be combined with price, class, rating, amenities, property type, or brand filters. |
| `amenities` | array of integer | No | Google Hotels amenity IDs. |
| `vacation_rentals` | boolean | No | Search vacation rentals instead of hotels. |
| `eco_certified` | boolean | No | Return only eco-certified hotels. Cannot be combined with other non-boolean filters. |
| `special_offers` | boolean | No | Return only properties with special offers. Cannot be combined with other non-boolean filters. |
| `property_types` | array of integer | No | Google Hotels property type IDs. |
| `brands` | array of integer | No | Google Hotels brand IDs. |
| `bedrooms` | integer | No | Minimum bedrooms for vacation rental searches. Constraints: minimum 1; maximum 20. |
| `bathrooms` | integer | No | Minimum bathrooms for vacation rental searches. Constraints: minimum 1; maximum 20. |
| `next_page_token` | string | No | Pagination token returned by a previous Google Hotels search response. |
| `property_token` | string | No | Specific Google Hotels property token, often returned by Google Hotels autocomplete. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Google Hotels. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-hotels-search-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Booking.com Hotel Details Scraper for Travel](https://apify.com/thescrappa/booking-hotel-details-scraper)
- [Booking.com Search Scraper for Travel Planning](https://apify.com/thescrappa/booking-search-scraper)
- [Google Flights Search Scraper for Travel Planning](https://apify.com/thescrappa/google-flights-search-scraper)
- [Google Hotels Autocomplete Scraper for Stays](https://apify.com/thescrappa/google-hotels-autocomplete-scraper)
- [Google Maps Directions Scraper for Travel Planning](https://apify.com/thescrappa/google-maps-directions-scraper)

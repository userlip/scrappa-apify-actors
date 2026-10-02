# Kayak Hotel Rates Scraper

Compare provider rate groups for KAYAK hotel IDs with room descriptions, total prices, and booking links. Omitted stay dates default to a future two-night visit.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `localizedTitle` | String | Localized heading for a group of room or provider rates. |
| `numToDisplay` | Integer | Number of rate rows KAYAK marks for display in this group. |
| `rows` | Array\<Object\> | Room descriptions and their provider booking options. |
| `input_hotel_id` | Integer | KAYAK hotel identifier submitted for this lookup. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Travel planners can compare room offers and booking providers for a hotel.
- Rate analysts can monitor total prices across future travel dates.
- Hospitality teams can track provider availability for selected properties.

## How to use

1. Add a KAYAK hotel ID to each entry in `hotels`.
2. Set stay dates and guest options or use the relative date defaults.
3. Review rate groups and provider booking links in the dataset.

```json
{
  "hotels": [
    {
      "hotel_id": 15297
    }
  ],
  "adults": 2,
  "rooms": 1,
  "price_mode": "total",
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `hotels` | Array\<object\> | Yes | KAYAK hotel identifiers whose provider rates you want to compare. |
| `hotels[].hotel_id` | integer | Yes per entry | KAYAK hotel identifier used to retrieve property rates or reviews. |
| `hotels[].checkin` | string | No | Hotel check-in date in YYYY-MM-DD format. When omitted, check-in is set to 30 days from today. |
| `hotels[].checkout` | string | No | Hotel check-out date in YYYY-MM-DD format, after check-in. When omitted, check-out is set to two days after check-in. |
| `hotels[].adults` | integer | No | Number of adult travelers included in the search. |
| `hotels[].rooms` | integer | No | Number of hotel rooms requested. |
| `hotels[].child_age` | integer | No | Age of one child in the hotel search, from 0 to 17. |
| `hotels[].price_mode` | string | No | Whether quoted prices are shown as totals or per-night or per-person values. Accepted values: total, nightly. |
| `checkin` | string | No | Hotel check-in date in YYYY-MM-DD format. When omitted, check-in is set to 30 days from today. |
| `checkout` | string | No | Hotel check-out date in YYYY-MM-DD format, after check-in. When omitted, check-out is set to two days after check-in. |
| `adults` | integer | No | Number of adult travelers included in the search. |
| `rooms` | integer | No | Number of hotel rooms requested. |
| `child_age` | integer | No | Age of one child in the hotel search, from 0 to 17. |
| `price_mode` | string | No | Whether quoted prices are shown as totals or per-night or per-person values. Accepted values: total, nightly. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "localizedTitle": "Standard room offers",
  "numToDisplay": 1,
  "rows": [
    {
      "localizedDescription": "Queen room, 2 guests",
      "bookingOptions": [
        {
          "price": {
            "price": 318,
            "currency": "USD",
            "localizedPrice": "$318"
          },
          "totalPrice": {
            "price": 318,
            "currency": "USD",
            "localizedPrice": "$318"
          },
          "rateType": "standard",
          "localizedProviderName": "Booking.com",
          "providerCode": "booking",
          "afterclickUrl": {
            "url": "https://www.booking.com/hotel/us/park-mgm.html",
            "urlType": "booking"
          }
        }
      ]
    }
  ],
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~kayak-hotel-rates-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### How do I get a KAYAK hotel ID?

Use Kayak Hotels Search Scraper and copy the `hid` value from a property result.

## Related Scrappa Actors

- [Kayak Hotels Search Scraper](https://apify.com/thescrappa/kayak-hotels-search-scraper)
- [Kayak Hotel Details Scraper](https://apify.com/thescrappa/kayak-hotel-details-scraper)
- [Kayak Hotel Reviews Scraper](https://apify.com/thescrappa/kayak-hotel-reviews-scraper)

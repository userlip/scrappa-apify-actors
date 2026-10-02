# Booking.com Rooms & Rates Scraper

Check Booking.com room options for hotel URLs with room names, prices, occupancy, bed descriptions, and availability. Omitted dates default to a future two-night stay.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `name` | String | Name displayed by the source for this record. |
| `price` | String | Quoted price in the currency shown by the source. |
| `occupancy` | String | Occupancy value associated with this booking.com rooms & rates record. |
| `beds` | String | Beds value associated with this booking.com rooms & rates record. |
| `available` | Boolean | Available value associated with this booking.com rooms & rates record. |
| `input_url` | String | Hotel or job page URL supplied for this lookup. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Travel planners can compare available room types before choosing a hotel.
- Hospitality analysts can monitor quoted room prices for selected travel dates.
- Booking teams can collect occupancy and bed details for lodging research.

## How to use

1. Add one Booking.com hotel URL per entry in `hotels`.
2. Set check-in and check-out dates or use the relative date defaults.
3. Choose guest counts and currency, then export the room rows.

```json
{
  "hotels": [
    {
      "url": "https://www.booking.com/hotel/de/adlon-kempinski-berlin.html"
    }
  ],
  "group_adults": 2,
  "no_rooms": 1,
  "lang": "en-us",
  "currency": "EUR",
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `hotels` | Array\<object\> | Yes | Hotel listing URLs whose room options and availability you want to inspect. |
| `hotels[].url` | string | Yes per entry | Public source page URL used to retrieve this record. |
| `hotels[].country` | string | No | Kununu country market code, such as de, at, or ch. |
| `hotels[].slug` | string | No | Hotel slug from the Booking.com URL \(e.g. ritz-paris\). The trailing .html is optional. |
| `hotels[].checkin` | string | No | Hotel check-in date in YYYY-MM-DD format. When omitted, check-in is set to 30 days from today. |
| `hotels[].checkout` | string | No | Hotel check-out date in YYYY-MM-DD format, after check-in. When omitted, check-out is set to two days after check-in. |
| `hotels[].group_adults` | integer | No | Number of adults \(1-30\). |
| `hotels[].group_children` | integer | No | Number of children \(0-20\). |
| `hotels[].no_rooms` | integer | No | Number of rooms \(1-30\). |
| `hotels[].lang` | string | No | UI language hint \(e.g. en-us, de\). |
| `hotels[].currency` | string | No | 3-letter currency code \(e.g. EUR, USD\). |
| `country` | string | No | Kununu country market code, such as de, at, or ch. |
| `slug` | string | No | Hotel slug from the Booking.com URL \(e.g. ritz-paris\). The trailing .html is optional. |
| `checkin` | string | No | Hotel check-in date in YYYY-MM-DD format. When omitted, check-in is set to 30 days from today. |
| `checkout` | string | No | Hotel check-out date in YYYY-MM-DD format, after check-in. When omitted, check-out is set to two days after check-in. |
| `group_adults` | integer | No | Number of adults \(1-30\). |
| `group_children` | integer | No | Number of children \(0-20\). |
| `no_rooms` | integer | No | Number of rooms \(1-30\). |
| `lang` | string | No | UI language hint \(e.g. en-us, de\). |
| `currency` | string | No | 3-letter currency code \(e.g. EUR, USD\). |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "name": "Deluxe King Room",
  "price": "€279",
  "occupancy": "2 guests",
  "beds": "1 king bed",
  "available": true,
  "input_url": "https://www.booking.com/hotel/de/adlon-kempinski-berlin.html",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~booking-rooms-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Are room prices guaranteed?

No. Prices and availability can change before booking and depend on the selected dates, guest count, and source response.

## Related Scrappa Actors

- [Booking.com Search Scraper](https://apify.com/thescrappa/booking-search-scraper)
- [Booking.com Hotel Details Scraper](https://apify.com/thescrappa/booking-hotel-details-scraper)
- [Booking.com Hotel Prices Scraper](https://apify.com/thescrappa/booking-prices-scraper)

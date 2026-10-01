# Booking.com Hotel Photos Scraper

Collect public photo URLs from Booking.com hotel pages, together with the hotel name and canonical property link. Add multiple hotels to build a lodging image reference list.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `url` | String | Public source or listing URL associated with this record. |
| `input_url` | String | Hotel or job page URL supplied for this lookup. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Travel publishers can gather image links for destination research.
- Hotel analysts can compare visual inventory across competing properties.
- Lodging platforms can enrich property records with public photo sources.

## How to use

1. Add a Booking.com hotel URL to each entry in `hotels`.
2. Choose language and currency hints if they matter to your workflow.
3. Export the returned photo URLs with the hotel identifier from each batch entry.

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
| `hotels` | Array\<object\> | Yes | Hotel listing URLs whose public photo URLs you want to collect. |
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
  "url": "https://cf.bstatic.com/xdata/images/hotel/max1024x768/berlin-lobby.jpg",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~booking-photos-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Does the Actor download image files?

No. It returns public photo URLs reported on the Booking.com property page.

## Related Scrappa Actors

- [Booking.com Hotel Details Scraper](https://apify.com/thescrappa/booking-hotel-details-scraper)
- [Booking.com Search Scraper](https://apify.com/thescrappa/booking-search-scraper)
- [Booking.com Hotel Facilities Scraper](https://apify.com/thescrappa/booking-facilities-scraper)

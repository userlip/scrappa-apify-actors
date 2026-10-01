# Booking.com Search Scraper

Compare Booking.com stays by property name, guest score, nightly price and review count. Choose destination, dates and guest counts to compare the stays returned for each search.

## What data can you extract?

Property details and nightly rates reflect the selected stay and the details Booking.com displays.

| Field | Type | Description |
| --- | --- | --- |
| `name` | text | Name of the hotel listing, as shown by Booking.com; null when no name is published. |
| `url` | link | Source page url for this hotel listing on Booking.com; null when the source does not provide a URL. |
| `image` | image | Image for this hotel listing on Booking.com; null when the source does not provide a URL. |
| `review_score` | number | Booking.com guest score for this hotel, on the source’s 1-to-10 scale; null when a property has no review score. |
| `review_score_word` | text | Booking.com label for the guest score, such as Wonderful or Very good; null when the source has no label. |
| `review_count` | number | Number of reviews shown by Booking.com, as a whole number; zero is possible, and null means no count was reported. |
| `location` | text | Location shown for the hotel listing by Booking.com, in the format used by the source; null when it is omitted. |
| `price` | text | Listed price for this hotel listing, as a numeric amount in the listing currency; null when Booking.com provides no price. |
| `currency` | text | Currency code for this hotel listing, formatted as Booking.com displays it, including the currency when shown; null when unavailable. |
| `request_search_index` | number | Search result index passed to Booking.com. This input value is copied into the output row; null when it was not supplied. |
| `request_ss` | text | Source search selector passed to Booking.com. This input value is copied into the output row; null when it was not supplied. |
| `request_checkin` | date | Check-in date passed to Booking.com; Use the date format listed in Input. This input value is copied into the output row; null when it was not supplied. |
| `request_checkout` | date | Check-out date passed to Booking.com; Use the date format listed in Input. This input value is copied into the output row; null when it was not supplied. |
| `request_group_adults` | number | Number of adult guests passed to Booking.com. This input value is copied into the output row; null when it was not supplied. |
| `request_group_children` | number | Number of child guests passed to Booking.com. This input value is copied into the output row; null when it was not supplied. |
| `request_no_rooms` | number | Number of rooms passed to Booking.com. This input value is copied into the output row; null when it was not supplied. |
| `request_lang` | text | Language code passed to Booking.com; Use a language code such as en or de. This input value is copied into the output row; null when it was not supplied. |
| `request_currency` | text | Three-letter currency code passed to Booking.com; Use a three-letter code such as USD or EUR. This input value is copied into the output row; null when it was not supplied. |

## Use cases

- Travelers can compare fares or nightly rates before choosing a trip.
- Travel teams can check public options across routes, destinations and dates.
- Researchers can track prices and ratings in a travel market.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `searches` and use the identifier or URL format required by Booking.com.
3. Start the run and open its default dataset to inspect or download the rows.

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

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

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

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "name": "Juniper House Hotel",
  "price": "€68.00",
  "review_count": 184,
  "location": "Seattle, WA",
  "url": "https://listings.example.com/record/731-alder-way",
  "image": "https://images.example.com/video/market-morning-thumb.jpg",
  "review_score": 4.7,
  "review_score_word": "Excellent"
}
```

## Pricing

**Current live price:** $0.20 per 1,000 results.

Each saved hotel record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~booking-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Which dates should I use for a Booking.com search?

Set `checkin` and `checkout` to the stay dates and provide guest and room counts. Use the `searches` batch input to compare several supported queries in one run.

## Related Scrappa Actors

- [Booking.com Hotel Details Scraper](https://apify.com/thescrappa/booking-hotel-details-scraper)
- [Google Flights Scraper](https://apify.com/thescrappa/google-flights-search-scraper)
- [Google Hotels Autocomplete Scraper](https://apify.com/thescrappa/google-hotels-autocomplete-scraper)
- [Google Hotels Search Scraper](https://apify.com/thescrappa/google-hotels-search-scraper)
- [Google Maps Directions Scraper](https://apify.com/thescrappa/google-maps-directions-scraper)

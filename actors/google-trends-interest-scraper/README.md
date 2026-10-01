# Google Trends Interest Scraper

Track relative Google Trends interest over time with dates and a score from 0 to 100. Choose a region, time range and search type to compare interest for one topic over time.

## What data can you extract?

Interest values are relative scores for the selected topic, region and period, not raw search counts.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | Result position in the Google Trends search-interest reading list, as a whole number; null when the source does not supply one. |
| `date` | text | Date attached to this Google Trends reading, in the source date format; null if the time bucket has no date. |
| `timestamp` | number | Unix timestamp in seconds for this Google Trends reading; null if the source does not supply one. |
| `value` | number | Google Trends interest for the selected topic and time bucket, normalized from 0 to 100. A score of 100 is the series peak; null means Google Trends supplied no reading. |
| `average` | number | Average normalized Google Trends interest across the returned time buckets, from 0 to 100; null when the series has no readings. |
| `max_value` | number | Highest normalized Google Trends interest score in the selected period, from 0 to 100; null when the series has no readings. |
| `min_value` | number | Lowest normalized Google Trends interest score in the selected period, from 0 to 100; null when the series has no readings. |
| `request_q` | text | Search phrase passed to Google Trends. This input value is copied into the output row; null when it was not supplied. |
| `request_geo` | text | Geographic target passed to Google Trends. This input value is copied into the output row; null when it was not supplied. |
| `request_time_range` | text | Time range passed to Google Trends. This input value is copied into the output row; null when it was not supplied. |
| `request_hl` | text | Interface language code passed to Google Trends; Use a language code such as en or de. This input value is copied into the output row; null when it was not supplied. |
| `request_search_type` | text | Search category passed to Google Trends. This input value is copied into the output row; null when it was not supplied. |
| `response_time_ms` | number | Response time for this Google Trends lookup, measured in milliseconds; null when no timing value was recorded. |

## Use cases

- SEO teams can compare relative search interest across topics, regions and selected periods before planning content.
- Researchers can spot seasonal peaks in search demand and choose when to refresh a campaign.
- Editors can compare interest series when deciding which topics deserve a dedicated page.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Enter a topic or search phrase, then adjust the locale, page or time range fields that this Actor supports.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "q": "tesla"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `q` | string | Yes | Keyword or phrase to analyze in Google Trends. |
| `geo` | string | No | Geographic location code, such as US, GB, DE, or Worldwide. |
| `time_range` | string | No | Time period for the trend timeline. Constraints: allowed values: 1h, 4h, 1d, 7d, 30d, 90d, 1y, 5y, all. |
| `hl` | string | No | Two-letter language code, such as en, de, es, or fr. |
| `search_type` | string | No | Google Trends vertical to analyze. Constraints: allowed values: web, images, news, youtube, shopping. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "date": "2026-09-25",
  "timestamp": 1790327700,
  "value": 64,
  "average": 48.5,
  "max_value": 91,
  "min_value": 7,
  "response_time_ms": 348
}
```

## Pricing

**Current live price:** $0.20 per 1,000 price points.

Each saved price point counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-trends-interest-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### What does a Google Trends value of 100 mean?

Google Trends normalizes interest from 0 to 100. A value of 100 marks the topic’s peak for the selected region and time range.

## Related Scrappa Actors

- [Google Trends Autocomplete Scraper](https://apify.com/thescrappa/google-trends-autocomplete-scraper)
- [Google Trends Related Queries Scraper](https://apify.com/thescrappa/google-trends-related-queries-scraper)
- [Google Images Scraper](https://apify.com/thescrappa/google-images-scraper)
- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)

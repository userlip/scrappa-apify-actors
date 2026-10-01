# Google News Scraper

Find Google News articles with headlines, publishers, publication dates and result links. Set a locale to focus the article list on a market.

## What data can you extract?

Headlines, publishers and dates follow the article cards Google News shows for the selected topic.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | Result position in the Google News news article list, as a whole number; null when the source does not supply one. |
| `title` | text | Title of the news article, as shown by Google News; null when no title is published. |
| `link` | link | Result link for this news article on Google News; null when the source does not provide a URL. |
| `source_name` | text | Source name shown for the news article by Google News, in the format used by the source; null when it is omitted. |
| `date` | text | Date shown for the news article shown by Google News, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `iso_date` | date | Calendar date for this news article shown by Google News, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `snippet` | text | Search snippet from Google News for this news article; null when the source has no text to show. |
| `thumbnail` | image | Thumbnail url for this news article on Google News; null when the source does not provide a URL. |
| `story_token` | text | Story token shown for the news article by Google News, in the format used by the source; null when it is omitted. |
| `request_q` | text | Search phrase passed to Google News. This input value is copied into the output row; null when it was not supplied. |
| `request_gl` | text | Two-letter country or region code passed to Google News; Use a two-letter country code such as us or de. This input value is copied into the output row; null when it was not supplied. |
| `request_hl` | text | Interface language code passed to Google News; Use a language code such as en or de. This input value is copied into the output row; null when it was not supplied. |

## Use cases

- News desks can collect headlines and publishers covering a current topic.
- Communications teams can monitor news coverage of a company or event.
- Researchers can compare article dates and source links.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Enter a topic or search phrase, then set the locale, page or result offset and sort order.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "queries": [
    "artificial intelligence"
  ],
  "q": "artificial intelligence",
  "page": 1
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | array of string | No | Recommended. Process many Google News keyword searches in one Apify run so run startup and storage overhead are shared across results. Leave empty when using token parameters. Constraints: maximum 10 items. |
| `q` | string | No | Backward-compatible single keyword or phrase to search in Google News. Prefer Search Queries for normal usage, especially when running more than one keyword. Leave empty when using a token parameter. |
| `gl` | string | No | Two-letter Google News country code, such as us, gb, de, or fr. |
| `hl` | string | No | Two-letter Google News interface language, such as en, de, es, or fr. |
| `page` | integer | No | Page number for pagination. Do not use together with Start Offset. Constraints: minimum 1. |
| `start` | integer | No | Zero-based result offset for pagination. Do not use together with Page. Constraints: minimum 0. |
| `so` | integer | No | Sort order: 0 for relevance, 1 for date. Constraints: minimum 0; maximum 1. |
| `topic_token` | string | No | Google News topic token. Cannot be used with Search Query. |
| `kgmid` | string | No | Google Knowledge Graph entity ID starting with /m/ or /g/. Must be used alone. |
| `publication_token` | string | No | Google News publication token. Cannot be used with Search Query. |
| `section_token` | string | No | Google News section token. Cannot be used with Search Query. |
| `story_token` | string | No | Google News story cluster token. Cannot be used with Search Query. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Coastal cities expand weekend transit service",
  "date": "1790294400000",
  "link": "https://search.example.com/results/market-guide",
  "source_name": "Northwest Daily",
  "iso_date": "2026-09-25",
  "snippet": "A guide to choosing containers, light and watering schedules for a compact balcony herb garden.",
  "thumbnail": "https://images.example.com/video/market-morning-thumb.jpg",
  "story_token": "Story token for the record on Google News"
}
```

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

Each saved dataset record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-news-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Google News search recent coverage of a topic?

Search a topic and set the locale or sort order to focus the article list. Publication dates and available articles vary by search.

## Related Scrappa Actors

- [Google Search Scraper](https://apify.com/thescrappa/google-search-scraper)
- [Google Videos Scraper](https://apify.com/thescrappa/google-videos-scraper)
- [YouTube Search Scraper](https://apify.com/thescrappa/youtube-api-search-data)

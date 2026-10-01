# Google Videos Scraper

Find Google Videos results with titles, video links, publishers and thumbnails. Set the locale and page options to browse more videos relevant to your phrase.

## What data can you extract?

Titles, publishers and video links follow Google Videos results for the selected phrase.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | Result position in the Google Videos video search result list, as a whole number; null when the source does not supply one. |
| `title` | text | Title of the video search result, as shown by Google Videos; null when no title is published. |
| `video_url` | link | Video url for this video search result on Google Videos; null when the source does not provide a URL. |
| `displayed_link` | text | Displayed domain shown for the video search result by Google Videos, in the format used by the source; null when it is omitted. |
| `thumbnail_url` | image | Thumbnail url for this video search result on Google Videos; null when the source does not provide a URL. |
| `snippet` | text | Search snippet from Google Videos for this video search result; null when the source has no text to show. |
| `duration` | text | Duration of this video search result, in the duration format shown by the source; null when Google Videos provides no timing information. |
| `date` | text | Date shown for the video search result shown by Google Videos, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `key_moments_count` | number | Number of key moments shown by Google Videos, as a whole number; zero is possible, and null means no count was reported. |
| `request_q` | text | Search phrase passed to Google Videos. This input value is copied into the output row; null when it was not supplied. |
| `request_page` | number | Requested result page number passed to Google Videos; A whole-number page number. This input value is copied into the output row; null when it was not supplied. |
| `request_start` | number | Start passed to Google Videos. This input value is copied into the output row; null when it was not supplied. |
| `request_gl` | text | Two-letter country or region code passed to Google Videos; Use a two-letter country code such as us or de. This input value is copied into the output row; null when it was not supplied. |
| `request_hl` | text | Interface language code passed to Google Videos; Use a language code such as en or de. This input value is copied into the output row; null when it was not supplied. |
| `request_google_domain` | text | Google domain for the selected market passed to Google Videos. This input value is copied into the output row; null when it was not supplied. |
| `request_tbs` | text | Google search time filter passed to Google Videos. This input value is copied into the output row; null when it was not supplied. |
| `request_safe` | text | Safe-search setting passed to Google Videos. This input value is copied into the output row; null when it was not supplied. |

## Use cases

- SEO teams can check which pages and domains appear for a query.
- Communications teams can monitor snippets and links for a brand or topic.
- Researchers can compare titles and domains across locales or repeat searches.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `queries` and use the identifier or URL format required by Google Videos.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "queries": [
    "coffee brewing tutorial"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | array of string | No | Recommended. Process many Google Videos keyword searches in one Apify run so run startup and storage overhead are shared across video results. Constraints: minimum 1 items; maximum 10 items. |
| `q` | string | No | Backward-compatible single keyword or phrase to search in Google Videos. Prefer Search Queries for normal usage, especially when running more than one keyword. |
| `page` | integer | No | Page number for pagination. Cannot be used together with start. Constraints: minimum 1. |
| `start` | integer | No | Zero-based Google result offset. Cannot be used together with page. Constraints: minimum 0. |
| `hl` | string | No | Two-letter interface language code, such as en, de, es, or fr. |
| `gl` | string | No | Two-letter country code for localized Google Videos results, such as us, gb, de, or jp. |
| `google_domain` | string | No | Google domain to query, such as google.com, google.de, or google.co.uk. |
| `location` | string | No | Location string for localized results. Cannot be used together with uule. |
| `uule` | string | No | Google UULE encoded location parameter. Cannot be used together with location. |
| `tbs` | string | No | Google tbs filter syntax, such as qdr:d for past day, qdr:w for past week, qdr:m for past month, or qdr:y for past year. |
| `safe` | string | No | Safe search filtering. Constraints: allowed values: active, off. |
| `filter` | integer | No | Enable or disable Google's duplicate/similar-result filtering. Constraints: minimum 0; maximum 1. |
| `nfpr` | integer | No | Set to 1 to exclude auto-corrected query results. Constraints: minimum 0; maximum 1. |
| `lr` | string | No | Restrict results to a language, such as lang_en. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Seattle neighborhood market guide",
  "date": "1790294400000",
  "video_url": "https://www.youtube.com/watch?v=aB3dE5fG7hJ",
  "displayed_link": "northstar.example/market-guide",
  "thumbnail_url": "https://images.example.com/video/market-morning-thumb.jpg",
  "snippet": "A guide to choosing containers, light and watering schedules for a compact balcony herb garden.",
  "duration": "4:18",
  "key_moments_count": 18
}
```

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

Each saved video, post or comment record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-videos-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Google Videos search return publisher and thumbnail details?

Yes, when Google Videos includes those values in a result. Search by a phrase and set supported locale or page options from Input.

## Related Scrappa Actors

- [Google Images Scraper](https://apify.com/thescrappa/google-images-scraper)
- [YouTube Search Scraper](https://apify.com/thescrappa/youtube-api-search-data)
- [YouTube Video Comments Scraper](https://apify.com/thescrappa/youtube-api-video-comments)
- [YouTube Transcript Scraper](https://apify.com/thescrappa/youtube-transcript-scraper)

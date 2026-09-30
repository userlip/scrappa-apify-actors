# Google Videos Scraper for Creator Research

The Google Videos Scraper for Creator Research collects public video metadata and engagement counts from Google Videos. Provide a search phrase or a short list of phrases; the actor saves source fields such as `position`, `title`, `video_url`, and `displayed_link` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Google Videos. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | # returned for this result. |
| `title` | text | Title returned for this result. |
| `video_url` | link | Video URL returned for this result. |
| `displayed_link` | text | Displayed Link returned for this result. |
| `thumbnail_url` | image | Thumbnail returned for this result. |
| `snippet` | text | Snippet returned for this result. |
| `duration` | text | Duration returned for this result. |
| `date` | text | Date returned for this result. |
| `key_moments_count` | number | Key Moments returned for this result. |
| `request_q` | text | Query returned for this result. |
| `request_page` | number | Page returned for this result. |
| `request_start` | number | Start returned for this result. |
| `request_gl` | text | Country returned for this result. |
| `request_hl` | text | Language returned for this result. |
| `request_google_domain` | text | Google Domain returned for this result. |
| `request_tbs` | text | TBS returned for this result. |
| `request_safe` | text | Safe Search returned for this result. |

## Use cases

- Collect public video metadata and engagement counts for video and creator research.
- Review returned titles, channels, timestamps, or engagement fields.
- Export video records to a content planning or analysis workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `queries` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "queries": [
    "coffee brewing tutorial"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "position": 42,
  "title": "Example result",
  "video_url": "https://example.com/result/1",
  "displayed_link": "https://example.com/result/1",
  "thumbnail_url": "https://example.com/image.jpg",
  "snippet": "Example public text.",
  "duration": "3:21"
}
```

## Input fields

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

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Google Videos. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-videos-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Images Scraper for Creator Research](https://apify.com/thescrappa/google-images-scraper)
- [YouTube Search Results Scraper for Video Analysis](https://apify.com/thescrappa/youtube-api-search-data)
- [YouTube Video Comments Scraper for Viewer Insights](https://apify.com/thescrappa/youtube-api-video-comments)
- [YouTube Transcript Scraper for Creator Research](https://apify.com/thescrappa/youtube-transcript-scraper)

# Google Translate Scraper for Localization

The Google Translate Scraper for Localization collects translated text and detected language data from Google Translate. Provide one or more items; the actor saves source fields such as `success`, `index`, `text`, and `translated_text` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Google Translate. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `success` | boolean | Success returned for this result. |
| `index` | number | # returned for this result. |
| `text` | text | Original Text returned for this result. |
| `translated_text` | text | Translated Text returned for this result. |
| `source` | text | Source returned for this result. |
| `target` | text | Target returned for this result. |
| `error` | text | Error returned for this result. |
| `status_code` | number | Status Code returned for this result. |

## Use cases

- Collect translated text and detected language data from Google Translate for data workflows.
- Review the structured fields returned for each result.
- Export the dataset or schedule recurring runs in Apify.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `items` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "items": [
    {
      "text": "Good morning",
      "source": "en",
      "target": "de"
    }
  ],
  "text": "Good morning"
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "success": true,
  "index": 42,
  "text": "Example public text.",
  "translated_text": "Example value",
  "source": "Example value",
  "target": "Example value",
  "error": "Example value"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `items` | array of object | No | Translate multiple text items in one Apify run. Each item must include text, source, and target. Constraints: minimum 1 items; maximum 100 items. |
| `text` | string | No | Single text to translate when items is not provided. |
| `source` | string | No | Single-item source language code. |
| `target` | string | No | Single-item target language code. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Google Translate. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-translate-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [YouTube Transcript Scraper for Creator Research](https://apify.com/thescrappa/youtube-transcript-scraper)
- [YouTube Search Results Scraper for Video Analysis](https://apify.com/thescrappa/youtube-api-search-data)
- [Google Search Results Scraper for SEO Research](https://apify.com/thescrappa/google-search-scraper)

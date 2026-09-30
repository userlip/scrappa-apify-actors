# Website Content Extractor Scraper for SEO Research

The Website Content Extractor Scraper for SEO Research collects page text, metadata, links, and content fields from website content extractor scraper. Provide one or more public URLs; the actor saves source fields such as `success`, `input_url`, `url`, and `final_url` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by website content extractor scraper. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `success` | boolean | Success returned for this result. |
| `input_url` | link | Input URL returned for this result. |
| `url` | link | Requested URL returned for this result. |
| `final_url` | link | Final URL returned for this result. |
| `response_type` | text | Response Type returned for this result. |
| `include_html` | boolean | Include HTML returned for this result. |
| `site_status_code` | number | Site Status returned for this result. |
| `title` | text | Title returned for this result. |
| `description` | text | Description returned for this result. |
| `body_text` | text | Body Text returned for this result. |
| `links_count` | number | Links returned for this result. |
| `emails_count` | number | Emails returned for this result. |
| `phone_numbers_count` | number | Phones returned for this result. |
| `images_count` | number | Images returned for this result. |
| `languages_detected` | array | Languages returned for this result. |
| `markdown` | text | Markdown returned for this result. |
| `markdown_length` | number | Markdown Length returned for this result. |
| `error` | text | Error returned for this result. |
| `error_type` | text | Error Type returned for this result. |
| `error_code` | text | Error Code returned for this result. |
| `status_code` | number | Scrappa Status returned for this result. |

## Use cases

- Collect page text, metadata, links, and content fields to support SEO research.
- Compare results across search terms, websites, or markets.
- Export the dataset to a content, keyword, or reporting workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `urls` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "urls": [
    "https://example.com"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "success": true,
  "input_url": "https://example.com/result/1",
  "url": "https://example.com/result/1",
  "final_url": "https://example.com/result/1",
  "response_type": "Example value",
  "include_html": true,
  "site_status_code": 42
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `urls` | array | No | Preferred batch input. Enter one or more page URLs to extract in a single actor run. Constraints: minimum 1 items; maximum 20 items. |
| `url` | string | No | Backward-compatible single URL input. Prefer URLs for batching. |
| `include_html` | boolean | No | Include raw page HTML in JSON responses. Ignored when Response Type is Markdown. |
| `response_type` | string | No | Choose JSON for structured extraction fields, or Markdown for clean page content. Constraints: allowed values: json, markdown. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from website content extractor scraper. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~website-content-extractor-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Domain Availability Checker for Business Sites](https://apify.com/thescrappa/domain-availability-checker)
- [Similarweb Traffic Analytics Scraper for SEO](https://apify.com/thescrappa/similarweb-traffic-analytics-scraper)
- [Google Search Results Scraper for SEO Research](https://apify.com/thescrappa/google-search-scraper)

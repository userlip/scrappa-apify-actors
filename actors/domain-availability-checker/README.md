# Domain Availability Checker for Business Sites

The Domain Availability Checker for Business Sites collects domain availability and registration status from domain availability checker. Provide one or more domain names; the actor saves source fields such as `success`, `domain`, `available`, and `registered` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by domain availability checker. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `success` | boolean | Success returned for this result. |
| `domain` | text | Domain returned for this result. |
| `available` | boolean | Available returned for this result. |
| `registered` | boolean | Registered returned for this result. |
| `status` | text | Status returned for this result. |
| `confidence` | text | Confidence returned for this result. |
| `source` | text | Source returned for this result. |
| `rdap_url` | link | RDAP URL returned for this result. |
| `rdap_status_code` | number | RDAP Status returned for this result. |
| `rdap_events` | array | RDAP Events returned for this result. |
| `nameservers` | array | Nameservers returned for this result. |
| `message` | text | Message returned for this result. |
| `error` | text | Error returned for this result. |
| `status_code` | number | Status Code returned for this result. |
| `input_domain` | text | Input Domain returned for this result. |

## Use cases

- Collect domain availability and registration status to support market research.
- Compare records across the input queries or entities you provide.
- Export structured results to research and reporting workflows.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `domains` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "domains": [
    "example.com"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "success": true,
  "domain": "Example value",
  "available": true,
  "registered": true,
  "status": "Example value",
  "confidence": "Example value",
  "source": "Example value"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `domains` | array of string | No | Domain names or URLs to check in one Apify run. Batch this field to reduce run overhead and get one dataset item per checked domain. Constraints: minimum 1 items. |
| `domain` | string | No | Backward-compatible single domain input. Prefer Domains for bulk checks. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from domain availability checker. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~domain-availability-checker/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Website Content Extractor Scraper for SEO Research](https://apify.com/thescrappa/website-content-extractor-scraper)
- [Similarweb Traffic Analytics Scraper for SEO](https://apify.com/thescrappa/similarweb-traffic-analytics-scraper)
- [Google Search Results Scraper for SEO Research](https://apify.com/thescrappa/google-search-scraper)

# Similarweb Traffic Analytics Scraper for SEO

The Similarweb Traffic Analytics Scraper for SEO collects website traffic, engagement, and channel metrics from Similarweb. Provide one or more domain names; the actor saves source fields such as `success`, `domain`, `site_name`, and `title` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Similarweb. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `success` | boolean | Success returned for this result. |
| `domain` | text | Domain returned for this result. |
| `site_name` | text | Site Name returned for this result. |
| `title` | text | Title returned for this result. |
| `category` | text | Category returned for this result. |
| `global_rank_value` | number | Global Rank returned for this result. |
| `country_rank_value` | number | Country Rank returned for this result. |
| `country_code` | text | Country returned for this result. |
| `category_rank_value` | number | Category Rank returned for this result. |
| `visits` | number | Visits returned for this result. |
| `time_on_site` | number | Time On Site returned for this result. |
| `page_per_visit` | number | Pages / Visit returned for this result. |
| `bounce_rate` | number | Bounce Rate returned for this result. |
| `traffic_direct` | number | Direct returned for this result. |
| `traffic_search` | number | Search returned for this result. |
| `traffic_social` | number | Social returned for this result. |
| `traffic_referrals` | number | Referrals returned for this result. |
| `traffic_mail` | number | Mail returned for this result. |
| `traffic_paid_referrals` | number | Paid Referrals returned for this result. |
| `latest_month` | text | Latest Month returned for this result. |
| `latest_month_visits` | number | Latest Visits returned for this result. |
| `top_countries` | object | Top Countries returned for this result. |
| `top_keywords` | object | Top Keywords returned for this result. |
| `estimated_monthly_visits` | object | Monthly Visits returned for this result. |
| `monthly_visits` | object | Raw Monthly Visits returned for this result. |
| `screenshot` | image | Screenshot returned for this result. |
| `request_domain` | text | Request Domain returned for this result. |
| `input_domain` | text | Input Domain returned for this result. |
| `status_code` | number | Status returned for this result. |
| `error` | text | Error returned for this result. |

## Use cases

- Collect website traffic, engagement, and channel metrics to support SEO research.
- Compare results across search terms, websites, or markets.
- Export the dataset to a content, keyword, or reporting workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `domains` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "domains": [
    "google.com"
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
  "site_name": "Example value",
  "title": "Example result",
  "category": "Example value",
  "global_rank_value": 42,
  "country_rank_value": 42
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `domain` | string | No | Single domain to analyze. URLs are accepted and normalized to the hostname. |
| `domains` | array of string | No | Batch of domains to analyze in one run. Each unique domain creates one dataset item. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Similarweb. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~similarweb-traffic-analytics-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Domain Availability Checker for Business Sites](https://apify.com/thescrappa/domain-availability-checker)
- [Website Content Extractor Scraper for SEO Research](https://apify.com/thescrappa/website-content-extractor-scraper)
- [Google Search Results Scraper for SEO Research](https://apify.com/thescrappa/google-search-scraper)

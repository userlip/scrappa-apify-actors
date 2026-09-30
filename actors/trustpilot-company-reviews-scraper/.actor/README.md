# Trustpilot Company Reviews Scraper for Marketing

The Trustpilot Company Reviews Scraper for Marketing collects reviews, ratings, and comment details from Trustpilot. Provide a search phrase or a short list of phrases; the actor saves source fields such as `consumer_name`, `rating`, `title`, and `text` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Trustpilot. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `consumer_name` | text | Reviewer returned for this result. |
| `rating` | number | Rating returned for this result. |
| `title` | text | Title returned for this result. |
| `text` | text | Review returned for this result. |
| `published_date` | date | Published returned for this result. |
| `language` | text | Language returned for this result. |
| `isVerified` | boolean | Verified returned for this result. |
| `reply` | object | Reply returned for this result. |
| `id` | text | Review ID returned for this result. |
| `review_source` | text | Source Array returned for this result. |
| `company_domain` | text | Company Domain returned for this result. |
| `request_page` | number | Page returned for this result. |
| `request_sort` | text | Sort returned for this result. |
| `request_rating` | text | Rating Filter returned for this result. |

## Use cases

- Collect reviews, ratings, and comment details to support reputation research.
- Compare records across the input queries or entities you provide.
- Export structured results to research and reporting workflows.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. Set the lookup fields to match the query or identifier you want to collect.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "company_domain": "amazon.com",
  "page": 1,
  "per_page": 20
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "consumer_name": "Example value",
  "rating": 4.7,
  "title": "Example result",
  "text": "Example public text.",
  "published_date": "2026-09-30T10:00:00Z",
  "language": "en",
  "isVerified": true
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `company_domain` | string | Yes | Company domain with or without protocol, for example amazon.com or https://www.amazon.com. |
| `locale` | string | No | Trustpilot locale used for the review page. Constraints: allowed values: da-DK, de-AT, de-CH, de-DE, en-AU, en-CA, en-GB, en-IE, en-NZ, en-US, es-ES, fi-FI, fr-BE, nl-BE, fr-FR, it-IT, ja-JP, nb-NO, nl-NL, pl-PL, pt-BR, pt-PT, sv-SE. |
| `page` | integer | No | First Trustpilot review page to fetch. Trustpilot limits public access to approximately the first 10 pages. Constraints: minimum 1; maximum 10. |
| `max_pages` | integer | No | Number of pages to fetch in this run. Page plus Max Pages cannot go beyond page 10. Constraints: minimum 1; maximum 10. |
| `per_page` | integer | No | Maximum reviews returned per page after Scrappa filtering. Constraints: minimum 1; maximum 100. |
| `sort` | string | No | Use recency for monitoring or relevance for representative samples. Constraints: allowed values: recency, relevance. |
| `rating` | string | No | Comma-separated ratings from 1 to 5, for example 1,2 for negative-review monitoring or 4,5 for positive reviews. |
| `verified` | boolean | No | Only return verified reviews. |
| `with_replies` | boolean | No | Only return reviews that include company replies. |
| `query` | string | No | Keyword to search in review title or text, such as refund, delivery, support, or warranty. |
| `date_posted` | string | No | Limit reviews by publication date. Constraints: allowed values: any, last_30_days, last_3_months, last_6_months, last_12_months. |
| `fields` | string | No | Optional comma-separated Scrappa response fields to include, such as reviews, relevantReviews, aiSummaryReviews, businessUnit.displayName, and pagination. Leave empty for the full response. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Trustpilot. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~trustpilot-company-reviews-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Maps Reviews Scraper for Local Reputation](https://apify.com/thescrappa/google-maps-reviews-scraper)
- [Jameda Reviews Scraper for Lead Research](https://apify.com/thescrappa/jameda-reviews-scraper)
- [Kununu Reviews Scraper for Campaign Research](https://apify.com/thescrappa/kununu-reviews-scraper)
- [TrustedShops Reviews Scraper for Campaign Research](https://apify.com/thescrappa/trustedshops-reviews-scraper)
- [Trusted Shops Search Scraper for Campaign Research](https://apify.com/thescrappa/trustedshops-search-scraper)

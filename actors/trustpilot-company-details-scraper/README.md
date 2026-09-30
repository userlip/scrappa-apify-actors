# Trustpilot Company Details Scraper for Marketing

The Trustpilot Company Details Scraper for Marketing collects public record details and identifying fields from Trustpilot. Provide the fields listed below; the actor saves source fields such as `company_name`, `company_domain`, `trust_score`, and `stars` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Trustpilot. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `company_name` | text | Company returned for this result. |
| `company_domain` | text | Domain returned for this result. |
| `trust_score` | number | TrustScore returned for this result. |
| `stars` | number | Stars returned for this result. |
| `review_count` | number | Reviews returned for this result. |
| `is_claimed` | boolean | Claimed returned for this result. |
| `is_verified` | boolean | Verified returned for this result. |
| `website_url` | link | Website returned for this result. |
| `profile_url` | link | Trustpilot Profile returned for this result. |
| `email` | text | Email returned for this result. |
| `phone` | text | Phone returned for this result. |
| `country` | text | Country returned for this result. |
| `country_code` | text | Country Code returned for this result. |
| `city` | text | City returned for this result. |
| `address` | text | Address returned for this result. |
| `category_names` | text | Categories returned for this result. |
| `category_slugs` | text | Category Slugs returned for this result. |
| `request_locale` | text | Locale returned for this result. |
| `response_source` | text | Source returned for this result. |
| `scraped_at` | date | Scraped At returned for this result. |

## Use cases

- Collect public record details and identifying fields to support reputation research.
- Compare records across the input queries or entities you provide.
- Export structured results to research and reporting workflows.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `company_domains` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "company_domains": [
    "trustpilot.com"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "company_name": "Example Company",
  "company_domain": "Example Company",
  "trust_score": 4.7,
  "stars": 42,
  "review_count": 42,
  "is_claimed": true,
  "is_verified": true
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `company_domain` | string | No | Single company domain with or without protocol, for example trustpilot.com or https://www.trustpilot.com. |
| `company_domains` | array | No | Batch of company domains to process in one run. Each domain produces one dataset item. |
| `locale` | string | No | Trustpilot locale used for the company profile page. Constraints: allowed values: da-DK, de-AT, de-CH, de-DE, en-AU, en-CA, en-GB, en-IE, en-NZ, en-US, es-ES, fi-FI, fr-BE, nl-BE, fr-FR, it-IT, ja-JP, nb-NO, nl-NL, pl-PL, pt-BR, pt-PT, sv-SE. |
| `fields` | string | No | Optional comma-separated Scrappa response fields to include if projection is enabled for this endpoint later, such as basic_info, ratings, categories, contact, social_media, and metadata. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Trustpilot. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~trustpilot-company-details-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Maps Reviews Scraper for Local Reputation](https://apify.com/thescrappa/google-maps-reviews-scraper)
- [Jameda Reviews Scraper for Lead Research](https://apify.com/thescrappa/jameda-reviews-scraper)
- [Kununu Reviews Scraper for Campaign Research](https://apify.com/thescrappa/kununu-reviews-scraper)
- [TrustedShops Reviews Scraper for Campaign Research](https://apify.com/thescrappa/trustedshops-reviews-scraper)
- [Trusted Shops Search Scraper for Campaign Research](https://apify.com/thescrappa/trustedshops-search-scraper)

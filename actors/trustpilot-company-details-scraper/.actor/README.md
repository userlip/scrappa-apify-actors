# Trustpilot Company Details Scraper

Review a Trustpilot company profile with domain, TrustScore, star rating and review count. Submit multiple company domains in one run to retrieve records for each target.

## What data can you extract?

Ratings and review details follow the public Trustpilot profile; review text and owner replies vary by record.

| Field | Type | Description |
| --- | --- | --- |
| `company_name` | text | Employer name attached to the business profile, as shown by Trustpilot; null when the listing does not identify its employer. |
| `company_domain` | text | Company domain shown for the business profile by Trustpilot, in the format used by the source; null when it is omitted. |
| `trust_score` | number | Trustpilot score for this business profile, on Trustpilot’s 1-to-5 scale; null when no score is shown. |
| `stars` | number | Star rating for this business profile, on Trustpilot’s 1-to-5 scale; null when no score is shown. |
| `review_count` | number | Number of reviews shown by Trustpilot, as a whole number; zero is possible, and null means no count was reported. |
| `is_claimed` | boolean | Whether the business has claimed its profile; false is a reported value, while null means Trustpilot provided no flag. |
| `is_verified` | boolean | Whether the source marks the profile as verified; false is a reported value, while null means Trustpilot provided no flag. |
| `website_url` | link | Website url for this business profile on Trustpilot; null when the source does not provide a URL. |
| `profile_url` | link | Profile url for this business profile on Trustpilot; null when the source does not provide a URL. |
| `email` | text | Public email shown by Trustpilot; null when the profile or listing does not publish contact details. |
| `phone` | text | Public phone shown by Trustpilot; null when the profile or listing does not publish contact details. |
| `country` | text | Country shown for the business profile by Trustpilot; null when Trustpilot does not provide the value. |
| `country_code` | text | Country code shown for the business profile by Trustpilot; null when Trustpilot does not provide the value. |
| `city` | text | City shown for the business profile by Trustpilot; null when Trustpilot does not provide the value. |
| `address` | text | Address shown for the business profile by Trustpilot, in the format used by the source; null when it is omitted. |
| `category_names` | text | Category names shown for the business profile by Trustpilot, in the format used by the source; null when it is omitted. |
| `category_slugs` | text | Category slugs shown for the business profile by Trustpilot, in the format used by the source; null when it is omitted. |
| `request_locale` | text | Language and region locale passed to Trustpilot. This input value is copied into the output row; null when it was not supplied. |
| `response_source` | text | Source used for the response shown for the business profile by Trustpilot, in the format used by the source; null when it is omitted. |
| `scraped_at` | date | Time the page was retrieved shown by Trustpilot, in ISO 8601 date and time; null if the source omits the date. |

## Use cases

- Teams can review source records before a follow-up decision.
- Researchers can compare available records across targets or runs.
- Analysts can use source links to maintain a focused dataset.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `company_domains` and use the identifier or URL format required by Trustpilot.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "company_domains": [
    "trustpilot.com"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `company_domain` | string | No | Single company domain with or without protocol, for example trustpilot.com or https://www.trustpilot.com. |
| `company_domains` | array | No | Batch of company domains to process in one run. Each domain produces one dataset item. |
| `locale` | string | No | Trustpilot locale used for the company profile page. Constraints: allowed values: da-DK, de-AT, de-CH, de-DE, en-AU, en-CA, en-GB, en-IE, en-NZ, en-US, es-ES, fi-FI, fr-BE, nl-BE, fr-FR, it-IT, ja-JP, nb-NO, nl-NL, pl-PL, pt-BR, pt-PT, sv-SE. |
| `fields` | string | No | Optional comma-separated Scrappa response fields to include if projection is enabled for this endpoint later, such as basic_info, ratings, categories, contact, social_media, and metadata. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "company_name": "Northstar Market Labs",
  "review_count": 184,
  "company_domain": "northstar.example",
  "trust_score": 4.6,
  "stars": 4.5,
  "is_claimed": true,
  "is_verified": true,
  "website_url": "https://northstar.example"
}
```

## Pricing

**Current live price:** $0.20 per 1,000 results.

Each completed profile or detail lookup counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~trustpilot-company-details-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Which identifier does Trustpilot Company Details need?

Submit a company domain, such as the domain used for its Trustpilot business profile. TrustScore, rating and review totals depend on the public profile.

## Related Scrappa Actors

- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)
- [Jameda Reviews Scraper](https://apify.com/thescrappa/jameda-reviews-scraper)
- [Kununu Reviews Scraper](https://apify.com/thescrappa/kununu-reviews-scraper)
- [TrustedShops Reviews Scraper](https://apify.com/thescrappa/trustedshops-reviews-scraper)
- [Trusted Shops Search Scraper](https://apify.com/thescrappa/trustedshops-search-scraper)

# Impressum Contact Data Extractor

Inspect a list of company domains for an Impressum or legal notice. Request company name, address, email, phone, and other supported fields, with values returned when the source provides them.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `success` | Boolean | True when the German legal notice request returned a successful response. |
| `data` | Object | German legal notice response payload for this legal notice record, including `company_name`, `address`, `email` and related source fields. |
| `error` | String or null | Error message or code returned by the request; null when the requested information was retrieved. |
| `error_code` | String or null | Error message or code returned by the request; null when the requested information was retrieved. |
| `impressum_url` | String | Public page URL identified as the website legal notice. |
| `phone` | String or null | Phone number requested from the company legal notice; null when the website does not publish it. |
| `company_name` | String | Name or title assigned to this German legal notice legal notice record. |
| `address` | String | Postal address published in the website legal notice. |
| `email` | String | Contact email address published in the website legal notice. |
| `input_domain` | String | Domain submitted to retrieve this legal notice record. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- B2B teams can prepare company records from public legal notices.
- Compliance analysts can check which contact fields a website publishes.
- Researchers can map legal notice pages and company details across domains.

## How to use

1. Add one website domain to `domains` for each lookup.
2. Choose the comma-separated fields and an optional language for extracted values.
3. Set `maxResults` and export one contact record per domain.

```json
{
  "domains": [
    {
      "domain": "scrappa.co"
    }
  ],
  "fields": "company_name,address,email,phone",
  "language": "de",
  "maxResults": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `domains` | Array\<object\> | Yes | Company website domains whose legal notice you want to inspect. |
| `domains[].domain` | string | Yes per entry | The domain to search for impressum \(e.g., example.com\) |
| `domains[].fields` | string | No | Comma-separated list of field names to extract \(e.g., company\_name,address,email,vat\_id\) |
| `domains[].language` | string | No | Language code for extracted values \(e.g., de, en, fr\). Country names and other localizable values will be returned in this language. |
| `fields` | string | Yes | Comma-separated list of field names to extract \(e.g., company\_name,address,email,vat\_id\) |
| `language` | string | No | Language code for extracted values \(e.g., de, en, fr\). Country names and other localizable values will be returned in this language. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "success": true,
  "data": {
    "company_name": "Morgenrot Handel GmbH",
    "address": "Hafenallee 17, 10115 Berlin",
    "email": "kontakt@morgenrot.invalid",
    "phone": "+49 30 555 0184"
  },
  "error": null,
  "error_code": null,
  "impressum_url": "https://morgenrot.invalid/impressum",
  "company_name": "Morgenrot Handel GmbH",
  "address": "Hafenallee 17, 10115 Berlin",
  "email": "kontakt@morgenrot.invalid",
  "phone": "+49 30 555 0184",
  "input_domain": "scrappa.co",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.50 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Set **maxResults** to cap the number of dataset items saved in one run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~impressum-extractor/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Are all requested fields always present?

No. A website may omit a field or use a different legal page format, so unavailable values can be null.

## Related Scrappa Actors

- [LinkedIn Company Scraper - $0.30/1k results](https://apify.com/thescrappa/linkedin-company-scraper)
- [Trustpilot Company Details Scraper](https://apify.com/thescrappa/trustpilot-company-details-scraper)
- [Trusted Shops Shop Profile Scraper](https://apify.com/thescrappa/trustedshops-shop-profile-scraper)

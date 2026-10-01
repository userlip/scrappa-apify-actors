# Similarweb Traffic Analytics Scraper

Review Similarweb traffic estimates, visit trends, referral shares and country breakdowns. Add several domains to compare estimated visits, traffic sources and audience locations together.

## What data can you extract?

Traffic figures are estimates from Similarweb, not audited visit totals.

| Field | Type | Description |
| --- | --- | --- |
| `success` | boolean | Whether the lookup completed successfully; false is a reported value, while null means Similarweb provided no flag. |
| `domain` | text | Domain shown for the website traffic report by Similarweb, in the format used by the source; null when it is omitted. |
| `site_name` | text | Name of the website traffic report, as shown by Similarweb; null when no name is published. |
| `title` | text | Title of the website traffic report, as shown by Similarweb; null when no title is published. |
| `category` | text | Category assigned to the website traffic report by Similarweb; null when Similarweb does not provide the value. |
| `global_rank_value` | number | Global website rank shown for the website traffic report by Similarweb, in the format used by the source; null when it is omitted. |
| `country_rank_value` | number | Country website rank shown for the website traffic report by Similarweb, in the format used by the source; null when it is omitted. |
| `country_code` | text | Country code shown for the website traffic report by Similarweb; null when Similarweb does not provide the value. |
| `category_rank_value` | number | Category website rank shown for the website traffic report by Similarweb, in the format used by the source; null when it is omitted. |
| `visits` | number | Visits shown for the website traffic report by Similarweb, in the format used by the source; null when it is omitted. |
| `time_on_site` | number | Duration of this website traffic report, in the duration format shown by the source; null when Similarweb provides no timing information. |
| `page_per_visit` | number | Pages per visit shown for the website traffic report by Similarweb, in the format used by the source; null when it is omitted. |
| `bounce_rate` | number | Bounce rate reported by Similarweb, as a percentage or share in the source format; null when no estimate is available. |
| `traffic_direct` | number | Direct traffic share reported by Similarweb, as a percentage or share in the source format; null when no estimate is available. |
| `traffic_search` | number | Search traffic share reported by Similarweb, as a percentage or share in the source format; null when no estimate is available. |
| `traffic_social` | number | Social traffic share reported by Similarweb, as a percentage or share in the source format; null when no estimate is available. |
| `traffic_referrals` | number | Referral traffic share reported by Similarweb, as a percentage or share in the source format; null when no estimate is available. |
| `traffic_mail` | number | Email traffic share reported by Similarweb, as a percentage or share in the source format; null when no estimate is available. |
| `traffic_paid_referrals` | number | Paid referral traffic share reported by Similarweb, as a percentage or share in the source format; null when no estimate is available. |
| `latest_month` | text | Latest month shown for the website traffic report by Similarweb, in the format used by the source; null when it is omitted. |
| `latest_month_visits` | number | Latest month visits shown for the website traffic report by Similarweb, in the format used by the source; null when it is omitted. |
| `top_countries` | array of objects | Country traffic shares with country code and visit share from Similarweb; an empty list when no entries are available. |
| `top_keywords` | array of objects | Search keywords with estimated volume from Similarweb; an empty list when no entries are available. |
| `estimated_monthly_visits` | object | Estimated monthly visits grouped by month; values are Similarweb estimates rather than audited totals. |
| `monthly_visits` | object | Estimated visits grouped by month from Similarweb; the object keys identify months and values are estimated visits. |
| `screenshot` | image | Screenshot for this website traffic report on Similarweb; null when the source does not provide a URL. |
| `request_domain` | text | Domain name passed to Similarweb. This input value is copied into the output row; null when it was not supplied. |
| `input_domain` | text | Domain name passed to Similarweb. This input value is copied into the output row; null when it was not supplied. |
| `status_code` | number | Http status code shown for the website traffic report by Similarweb, in the format used by the source; null when it is omitted. |
| `error` | text | Diagnostic text for the Similarweb lookup; null when the request completes without an error. |

## Use cases

- SEO teams can compare estimated visits, traffic sources and engagement across domains.
- Researchers can review country and keyword estimates while sizing competitors.
- Analysts can track traffic estimates in a recurring domain report.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `domains` and use the identifier or URL format required by Similarweb.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "domains": [
    "google.com"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `domain` | string | No | Single domain to analyze. URLs are accepted and normalized to the hostname. |
| `domains` | array of string | No | Batch of domains to analyze in one run. Each unique domain creates one dataset item. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "domain": "northstar.example",
  "site_name": "Northstar Market Labs",
  "global_rank_value": 128.5,
  "monthly_visits": {
    "2026-08-01": 1284000,
    "2026-09-01": 1352000
  },
  "top_countries": [
    {
      "country_code": "US",
      "share": 0.24
    },
    {
      "country_code": "CA",
      "share": 0.11
    }
  ],
  "top_keywords": [
    {
      "keyword": "retail inventory software",
      "volume": 18400
    },
    {
      "keyword": "store planning tools",
      "volume": 9600
    }
  ],
  "title": "A practical guide to independent neighborhood shops",
  "category": "Business Services"
}
```

## Pricing

**Current live price:** $0.20 per 1,000 results.

Each saved dataset record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~similarweb-traffic-analytics-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Similarweb Traffic Analytics compare multiple domains?

Yes. Submit one or more website domains through the batch input. Visits, traffic sources and country values are estimates from Similarweb, not audited counts.

## Related Scrappa Actors

- [Domain Availability Checker](https://apify.com/thescrappa/domain-availability-checker)
- [Website Content Extractor](https://apify.com/thescrappa/website-content-extractor-scraper)
- [Google Search Scraper](https://apify.com/thescrappa/google-search-scraper)

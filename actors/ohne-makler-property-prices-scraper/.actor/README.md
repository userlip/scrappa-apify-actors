# Ohne-Makler Property Prices Scraper

Retrieve residential price-report sections from Ohne-Makler by German state slug. The response contains headings, tables, available charts, and locality information for the report.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `success` | Boolean | True when the Ohne-Makler request returned a successful response. |
| `market` | String | Report type for local residential asking prices. |
| `state` | String | The source location name or code used to place this price report on Ohne-Makler. |
| `city` | String or null | The source location name or code used to place this price report on Ohne-Makler. |
| `title` | String | Name or title assigned to this Ohne-Makler price report. |
| `intro` | String | Introductory text summarizing the report coverage and market. |
| `sections` | Array\<Object\> | Ordered Ohne-Makler report sections with headings, rent or price tables, chart labels, and region rows. |
| `input_state` | String | State submitted to retrieve this price report. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Housing analysts can compare state-level asking price information.
- Property investors can review tables and charts before selecting a market.
- Real estate writers can collect structured report sections for regional briefs.

## How to use

1. Add one German state slug to `states` per report.
2. Optionally include a city slug when you need local tables and charts.
3. Set `maxResults` and export each report as one dataset record.

```json
{
  "states": [
    {
      "state": "berlin"
    }
  ],
  "maxResults": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `states` | Array\<object\> | Yes | German state URL slugs for Ohne-Makler residential price reports. |
| `states[].state` | string | Yes per entry | Bundesland slug \(e.g. baden-wurttemberg\). |
| `states[].city` | string | No | Optional city slug for locality price tables and charts. |
| `city` | string | No | Optional city slug for locality price tables and charts. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "success": true,
  "market": "immobilienpreise",
  "state": "berlin",
  "city": null,
  "title": "Residential prices in Berlin",
  "intro": "Residential asking prices and local indicators for the Berlin market.",
  "sections": [
    {
      "heading": "Apartment asking prices",
      "tables": [
        {
          "headers": [
            "Property type",
            "Median asking price"
          ],
          "rows": [
            [
              "Apartment",
              "4,850 EUR per m²"
            ],
            [
              "House",
              "5,420 EUR per m²"
            ]
          ]
        }
      ],
      "charts": [
        "Median asking price by property type"
      ],
      "regions": [
        {
          "path": "berlin-mitte",
          "name": "Berlin Mitte"
        }
      ]
    }
  ],
  "input_state": "berlin",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~ohne-makler-property-prices-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can I request city-level information?

Yes. Add a city slug alongside the state to request locality price tables and charts when the report provides them.

## Related Scrappa Actors

- [Ohne-Makler Rent Index Scraper](https://apify.com/thescrappa/ohne-makler-rent-index-scraper)
- [Ohne-Makler Property Search Scraper](https://apify.com/thescrappa/ohne-makler-search-scraper)
- [ImmobilienScout24 Price Insights Scraper](https://apify.com/thescrappa/immobilienscout24-price-insights-scraper)

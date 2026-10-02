# Ohne-Makler Rent Index Scraper

Retrieve an Ohne-Makler Mietspiegel by city and state slug. The record contains rent-reference sections, tables, available chart information, and locality details.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `success` | Boolean | True when the Ohne-Makler request returned a successful response. |
| `market` | String | Report type identifying the local residential rent index. |
| `state` | String | The source location name or code used to place this rent index on Ohne-Makler. |
| `city` | String | The source location name or code used to place this rent index on Ohne-Makler. |
| `title` | String | Name or title assigned to this Ohne-Makler rent index. |
| `intro` | String | Introductory text describing the rent index coverage and market. |
| `sections` | Array\<Object\> | Ordered Ohne-Makler report sections with headings, rent or price tables, chart labels, and region rows. |
| `input_city` | Object | City and state slugs supplied for this rent-index lookup. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Renters can review local rent reference tables before comparing housing costs.
- Property owners can compare city-level rent figures across German markets.
- Housing analysts can build a dataset of rent-index sections by city.

## How to use

1. Add each city and its state slug to `cities`.
2. Set `maxResults` to cap the number of city reports saved.
3. Export each Mietspiegel response as one dataset record.

```json
{
  "cities": [
    {
      "city": "berlin",
      "state": "berlin"
    }
  ],
  "maxResults": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `cities` | Array\<object\> | Yes | City and state slugs used to look up an Ohne-Makler rent index. |
| `cities[].city` | string | Yes per entry | City slug \(e.g. abtsgmund\). |
| `cities[].state` | string | Yes per entry | Bundesland slug \(e.g. baden-wurttemberg\). |
| `state` | string | No | Bundesland slug \(e.g. baden-wurttemberg\). |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "success": true,
  "market": "mietspiegel",
  "state": "berlin",
  "city": "berlin",
  "title": "Mietspiegel Berlin 2026",
  "intro": "Cold rent values by apartment size for the Berlin market.",
  "sections": [
    {
      "heading": "Apartments by size",
      "tables": [
        {
          "headers": [
            "Apartment size",
            "Cold rent per m²"
          ],
          "rows": [
            [
              "50 to 70 m²",
              "12.80 EUR"
            ],
            [
              "70 to 90 m²",
              "13.40 EUR"
            ]
          ]
        }
      ],
      "charts": [
        "Median rent by apartment size"
      ],
      "regions": [
        {
          "path": "berlin-mitte",
          "name": "Berlin Mitte"
        }
      ]
    }
  ],
  "input_city": {
    "city": "berlin",
    "state": "berlin"
  },
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~ohne-makler-rent-index-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Why does every city need a state?

The source identifies Mietspiegel pages with both a city slug and its state slug, so include both values in each batch entry.

## Related Scrappa Actors

- [Ohne-Makler Property Prices Scraper](https://apify.com/thescrappa/ohne-makler-property-prices-scraper)
- [Ohne-Makler Property Search Scraper](https://apify.com/thescrappa/ohne-makler-search-scraper)
- [Immowelt Property Search Scraper](https://apify.com/thescrappa/immowelt-property-search-scraper)

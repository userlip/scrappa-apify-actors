# Semrush Trending Websites Scraper

Collect websites ranked in Semrush trending lists with estimated visits, device splits, and traffic growth. Search multiple country markets in one run and compare domains.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `domain` | String | Website hostname included in the Semrush trending list. |
| `total_traffic` | Integer | Estimated visits for the listed domain in the selected market. |
| `desktop_percent` | Number | Share of visits attributed to desktop devices, as a percentage. |
| `desktop_traffic` | Integer | Estimated desktop visits for the domain. |
| `mobile_percent` | Number | Share of visits attributed to mobile devices, as a percentage. |
| `mobile_traffic` | Integer | Estimated mobile visits for the domain. |
| `mom` | Number | Month-over-month percentage change in estimated traffic. |
| `yoy` | Number | Year-over-year percentage change in estimated traffic. |
| `top_source` | String | Leading traffic source reported for the website. |
| `input_country` | String | Country code used to select the Semrush trending market. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- SEO teams can identify websites gaining traffic in selected markets.
- Market researchers can monitor domain leaders across countries.
- Competitive analysts can compare estimated growth and device mix.

## How to use

1. Add one ISO country code to `countries` for each market.
2. Optionally set `category` to narrow the industry list.
3. Use `maxResults` to cap saved website rows across all markets.

```json
{
  "countries": [
    {
      "country": "us"
    }
  ],
  "maxResults": 20
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `countries` | Array\<object\> | Yes | Semrush country codes to retrieve trending websites for. |
| `countries[].country` | string | Yes per entry | Country sent to the source search. |
| `countries[].category` | string | No | Category sent to the source search. |
| `category` | string | No | Category sent to the source search. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "domain": "canvas.com",
  "total_traffic": 28500000,
  "desktop_percent": 61.5,
  "desktop_traffic": 17527500,
  "mobile_percent": 38.5,
  "mobile_traffic": 10972500,
  "mom": 8.4,
  "yoy": 24.7,
  "top_source": "direct",
  "input_country": "us",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Set **maxResults** to cap the number of dataset items saved in one run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~semrush-trending-websites-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Which countries can I search?

Use a two-letter country code supported by Semrush, such as `us`, `gb`, or `de`.

## Related Scrappa Actors

- [Semrush Domain Overview Scraper](https://apify.com/thescrappa/semrush-domain-overview-scraper)
- [Semrush Competitors Scraper](https://apify.com/thescrappa/semrush-competitors-scraper)
- [Google Trends Interest Scraper](https://apify.com/thescrappa/google-trends-interest-scraper)

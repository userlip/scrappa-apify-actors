# Semrush Keyword Suggestions Scraper

Expand Semrush seed phrases into related keyword ideas. Each suggestion is saved as a row with the original query so you can compare topics and export a search plan.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `value` | String | Related keyword phrase returned for the submitted Semrush seed. |
| `input_q` | String | Seed keyword submitted to Semrush to retrieve related suggestions. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- SEO writers can expand target terms into related phrases for briefs.
- Search marketers can compare keyword ideas across product categories.
- Content strategists can build topic lists from a set of seed queries.

## How to use

1. Add each seed phrase to `queries`.
2. Set `maxResults` to limit the total suggestions saved for the run.
3. Group exported suggestions by `input_q` to retain their source phrase.

```json
{
  "queries": [
    {
      "q": "notion"
    }
  ],
  "maxResults": 20
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | Array\<object\> | Yes | Keywords to expand with Semrush suggestions. |
| `queries[].q` | string | Yes per entry | Search Query sent to the source search. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "value": "notion pricing",
  "input_q": "notion",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~semrush-keyword-suggestions-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Does the Actor include search volume?

No. This endpoint returns suggestion phrases; it does not provide keyword volume metrics.

## Related Scrappa Actors

- [Google Trends Autocomplete Scraper](https://apify.com/thescrappa/google-trends-autocomplete-scraper)
- [Semrush Domain Overview Scraper](https://apify.com/thescrappa/semrush-domain-overview-scraper)
- [Baidu Autocomplete Scraper](https://apify.com/thescrappa/baidu-autocomplete-scraper)

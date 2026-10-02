# Google Autocomplete Keyword Scraper

Collect Google autocomplete phrases for seed queries and save one row for each suggestion. Add multiple searches and choose a language to compare related wording.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `value` | String | Autocomplete phrase suggested for the submitted Google query. |
| `input_query` | String | Google seed phrase submitted to retrieve autocomplete suggestions. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- SEO specialists can find long-tail phrases related to target keywords.
- Writers can discover follow-up questions and wording for article briefs.
- Product marketers can compare search phrasing across topics and languages.

## How to use

1. Add each seed phrase to `queries` as a separate item.
2. Choose a two-letter language code with `hl` when needed.
3. Set `limit` and `maxResults` to bound suggestions and saved rows.

```json
{
  "queries": [
    {
      "query": "best crm"
    }
  ],
  "hl": "en",
  "limit": 10,
  "maxResults": 20
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | Array\<object\> | Yes | Google search phrases to expand with autocomplete. |
| `queries[].query` | string | Yes per entry | Search Query sent to the source search. |
| `queries[].hl` | string | No | Language sent to the source search. |
| `queries[].limit` | integer | No | Suggestion Limit sent to the source search. |
| `hl` | string | No | Language sent to the source search. |
| `limit` | integer | No | Suggestion Limit sent to the source search. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "value": "best crm software",
  "input_query": "best crm",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~google-autocomplete-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### How many suggestions can one query return?

Set `limit` from 1 to 10. The source may return fewer suggestions for some phrases.

## Related Scrappa Actors

- [Google Trends Autocomplete Scraper](https://apify.com/thescrappa/google-trends-autocomplete-scraper)
- [Baidu Autocomplete Scraper](https://apify.com/thescrappa/baidu-autocomplete-scraper)
- [Google Search Scraper](https://apify.com/thescrappa/google-search)

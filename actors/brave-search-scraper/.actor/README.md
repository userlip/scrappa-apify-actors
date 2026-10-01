# Brave Search Scraper

Collect Brave result positions, links, snippets, redirect links, and displayed domains for a batch of phrases. Optional language and HTML inputs refine collection.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `position` | Integer | Brave search-result rank for the page, starting at 1. |
| `title` | String | Page title displayed by Brave Search. |
| `link` | String | Destination page URL for the Brave result. |
| `redirect_link` | String | Source redirect URL supplied for the result link. |
| `displayed_link` | String | Shortened host or address shown in the Brave result. |
| `snippet` | String | Text excerpt Brave Search displays for the page. |
| `source` | String | Publisher or website label attached to the result. |
| `input_query` | String | Search phrase submitted to Brave for this result set. |
| `scraped_at` | String | UTC date and time when these Brave search results were collected. |

## Use cases

- SEO teams can compare Brave result positions and snippets across queries.
- Editorial teams can gather publisher links for a topic.
- Search analysts can monitor page visibility on Brave.

## How to use

1. Add one phrase to `queries` for each search.
2. Choose a language or request HTML when needed.
3. Run the Actor and export result rows from the dataset.

```json
{
  "queries": [
    {
      "query": "best crm software"
    }
  ],
  "maxResults": 20
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | Array\<object\> | Yes | Brave search phrases. Add one object per phrase. |
| `queries[].query` | string | Yes per entry | Search query \(max 500 characters\). |
| `queries[].language` | string | No | Language filter. Accepts english/deutsch/... or ISO codes such as en, de. Default: all. |
| `queries[].page` | integer | No | Result page number \(1-10\). Default: 1. |
| `queries[].include_html` | boolean | No | Set to true to include the raw Brave HTML in the response. Default: false. |
| `queries[].q` | string | No | Alias for query. Prefer query for new integrations. |
| `language` | string | No | Language filter. Accepts english/deutsch/... or ISO codes such as en, de. Default: all. |
| `page` | integer | No | Result page number \(1-10\). Default: 1. |
| `include_html` | boolean | No | Set to true to include the raw Brave HTML in the response. Default: false. |
| `q` | string | No | Alias for query. Prefer query for new integrations. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "position": 1,
  "title": "Heat Island Cooling Strategies",
  "link": "https://www.epa.gov/heatislands/heat-island-cooling-strategies",
  "redirect_link": "https://www.epa.gov/heatislands/heat-island-cooling-strategies",
  "displayed_link": "epa.gov/heatislands",
  "snippet": "Trees, reflective surfaces, and green roofs can help lower urban temperatures.",
  "source": "U.S. Environmental Protection Agency",
  "input_query": "best crm software",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~brave-search-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Is raw result HTML included by default?

No. Set `include_html` to true only when your workflow needs the source HTML response.

## Related Scrappa Actors

- [Bing Search Scraper](https://apify.com/thescrappa/bing-search-scraper)
- [Baidu Search Scraper](https://apify.com/thescrappa/baidu-search-scraper)
- [Google Lens Visual Search Scraper](https://apify.com/thescrappa/google-lens-scraper)

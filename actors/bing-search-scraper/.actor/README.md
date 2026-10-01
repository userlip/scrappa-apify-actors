# Bing Search Scraper

Collect ranked Bing web results for a batch of phrases. Optional language, domain, file type, and SafeSearch filters focus each search.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `position` | Integer | Bing search-result rank for the page, starting at 1. |
| `title` | String | Page title shown in the Bing search result. |
| `description` | String | Snippet text Bing displays beneath the matching page title. |
| `url` | String | Destination URL for the page returned by Bing. |
| `domain` | String | Website host for this Bing result, without its page path. |
| `input_query` | String | Search phrase submitted to Bing for this result set. |
| `scraped_at` | String | UTC date and time when these Bing search results were collected. |

## Use cases

- SEO managers can monitor positions and snippets for target queries.
- Content teams can collect useful publisher pages and source domains.
- Competitor analysts can compare which sites appear for commercial searches.

## How to use

1. Add one phrase per object in `queries`.
2. Set optional filters such as `site`, `filetype`, `safe`, or `hl`.
3. Use `maxResults` to cap saved rows across the batch.

```json
{
  "queries": [
    {
      "query": "python tutorial"
    }
  ],
  "maxResults": 20
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | Array\<object\> | Yes | Bing search phrases. Add one object per phrase. |
| `queries[].query` | string | Yes per entry | Search query \(max 500 characters\). |
| `queries[].page` | integer | No | Page number \(1-50\). |
| `queries[].num` | integer | No | Results per page \(1-50\). |
| `queries[].site` | string | No | Restrict results to this domain, e.g. example.com. |
| `queries[].filetype` | string | No | Restrict results to this file extension, e.g. pdf. |
| `queries[].safe` | string | No | SafeSearch level: off, moderate or strict. |
| `queries[].hl` | string | No | Language code, e.g. en or en-GB. |
| `queries[].include_html` | boolean | No | Set to true to include the raw HTML in the response. Default: false. |
| `page` | integer | No | Page number \(1-50\). |
| `num` | integer | No | Results per page \(1-50\). |
| `site` | string | No | Restrict results to this domain, e.g. example.com. |
| `filetype` | string | No | Restrict results to this file extension, e.g. pdf. |
| `safe` | string | No | SafeSearch level: off, moderate or strict. |
| `hl` | string | No | Language code, e.g. en or en-GB. |
| `include_html` | boolean | No | Set to true to include the raw HTML in the response. Default: false. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "position": 1,
  "title": "Getting Started",
  "description": "Learn Python basics, from installing the interpreter to writing your first program.",
  "url": "https://www.python.org/about/gettingstarted/",
  "domain": "python.org",
  "input_query": "python tutorial",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~bing-search-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can I restrict results to one website?

Yes. Set `site` to a domain such as `example.com` to focus the query on that site.

## Related Scrappa Actors

- [Brave Search Scraper](https://apify.com/thescrappa/brave-search-scraper)
- [Baidu Search Scraper](https://apify.com/thescrappa/baidu-search-scraper)
- [Google Scholar Scraper](https://apify.com/thescrappa/google-scholar-scraper)

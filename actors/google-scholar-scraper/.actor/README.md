# Google Scholar Scraper

Collect ranked Scholar results for research topics. Rows include paper titles, result IDs, publication summaries, snippets, links, and available resources.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `position` | Integer | Position of the paper in Google Scholar results, starting at 1. |
| `title` | String | Paper title displayed by Google Scholar. |
| `result_id` | String | Google Scholar identifier associated with the paper record. |
| `link` | String | Google Scholar or publisher URL for the paper. |
| `snippet` | String | Short paper excerpt or matching text shown in Scholar results. |
| `publication_info` | Object | Publication venue, authors, and year displayed with the paper. |
| `resources` | Array\<object\> | Publisher and document links associated with the paper. |
| `input_q` | String | Research phrase submitted to Google Scholar for this result set. |
| `scraped_at` | String | UTC date and time when this paper record was collected. |

## Use cases

- Researchers can collect paper details and publisher links by topic.
- Librarians can build reading lists from ranked Google Scholar results.
- Research teams can monitor publication coverage for recurring queries.

## How to use

1. Add one research phrase per item in `queries`.
2. Set `maxPages` to bound collection and `maxResults` to cap saved papers.
3. Review paper titles, publication details, links, and resources.

```json
{
  "queries": [
    {
      "q": "transformer attention"
    }
  ],
  "maxResults": 20,
  "maxPages": 2
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | Array\<object\> | Yes | Academic search phrases. Add one object per phrase. |
| `queries[].q` | string | Yes per entry | Scholar search query. Optional when cites or cluster is supplied. |
| `queries[].hl` | string | No | Two-letter interface language, optionally with a region. Defaults to en. |
| `queries[].start` | integer | No | Zero-based result offset from 0 to 990. Defaults to 0. |
| `queries[].num` | integer | No | Results per request from 1 to 20. Defaults to 10. |
| `queries[].as_ylo` | integer | No | Earliest publication year to include. |
| `queries[].as_yhi` | integer | No | Latest publication year to include. |
| `queries[].scisbd` | integer | No | 0 for relevance, 1 for recent abstracts, or 2 for all recent additions. Allowed values: 0, 1, 2. |
| `queries[].cites` | string | No | Numeric publication ID for a cited-by search. May be combined with q. |
| `queries[].cluster` | string | No | Numeric publication ID for an all-versions search. Must be used alone. |
| `hl` | string | No | Two-letter interface language, optionally with a region. Defaults to en. |
| `start` | integer | No | Zero-based result offset from 0 to 990. Defaults to 0. |
| `num` | integer | No | Results per request from 1 to 20. Defaults to 10. |
| `as_ylo` | integer | No | Earliest publication year to include. |
| `as_yhi` | integer | No | Latest publication year to include. |
| `scisbd` | integer | No | 0 for relevance, 1 for recent abstracts, or 2 for all recent additions. Allowed values: 0, 1, 2. |
| `cites` | string | No | Numeric publication ID for a cited-by search. May be combined with q. |
| `cluster` | string | No | Numeric publication ID for an all-versions search. Must be used alone. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "position": 1,
  "title": "How Search Interfaces Shape Literature Reviews",
  "result_id": "GS-2025-01842",
  "link": "https://scholar.google.com/scholar?cluster=1748293102842",
  "snippet": "This study compares citation discovery and screening workflows across digital research libraries.",
  "publication_info": {
    "summary": "Journal of Digital Scholarship, 2025",
    "authors": [
      "Priya Menon",
      "Rafael Costa"
    ]
  },
  "resources": [
    {
      "title": "Open access article",
      "file_format": "PDF",
      "link": "https://repository.uni-muenster.de/document/1842"
    }
  ],
  "input_q": "transformer attention",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Pagination follows the source response. Set **maxPages** per batch entry and **maxResults** across the run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items and **maxPages** to limit pages for each entry. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~google-scholar-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Does the Actor download full papers?

No. It returns result metadata and resource links exposed in search results; publisher access rules still apply.

## Related Scrappa Actors

- [Google AI Overview Scraper](https://apify.com/thescrappa/google-ai-overview-scraper)
- [Bing Search Scraper](https://apify.com/thescrappa/bing-search-scraper)
- [Google Shopping Scraper](https://apify.com/thescrappa/google-shopping-scraper)

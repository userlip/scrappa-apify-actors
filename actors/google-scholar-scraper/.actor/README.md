# Google Scholar Scraper

Search Google Scholar for academic papers, citations, author details, and publication links.

## Data you get

- **title**: Result title or listing name.
- **link**: Result destination URL.
- **snippet**: Text excerpt shown with the result.
- **publication_info**: Publication, authors, and citation context.
- **result_id**: Stable result identifier when provided.

## Use cases

- Literature discovery
- Citation research
- Academic landscape analysis

## How to use

Add one or more entries to **queries**. Each entry maps its **q** value to the Scrappa **q** input. Shared endpoint options can be set at the top level.

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

## Output example

This synthetic example shows the response fields and the input value attached to each result.

```json
{
  "position": 1,
  "title": "An Example Study of Attention Models",
  "result_id": "example-paper-1",
  "link": "https://example.com/paper",
  "snippet": "A synthetic paper abstract for the fixture.",
  "publication_info": {
    "summary": "Example Journal, 2025",
    "authors": [
      "Sample Author"
    ]
  },
  "resources": [],
  "input_q": "transformer attention",
  "scraped_at": "2026-01-01T00:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. The Actor writes one dataset item for each result.

This Actor supports pagination and stops at the configured **maxPages** or **maxResults** limit.

## FAQ

### Is scraping this data legal?

Scraping rules depend on the source, the data, and how you use it. Review the applicable laws, source terms, and privacy requirements for your use case. You are responsible for your collection and use of the data.

### Are there request limits?

You can submit up to 100 batch entries per run. Set **maxResults** to cap saved rows and **maxPages** to bound pagination for each entry. Scrappa API limits and source availability also apply.

### Can I use the output with integrations or the API?

Yes. Read results from the Apify dataset, use the Apify API or client libraries, or connect the dataset to your existing data workflow. Each row includes **input_q** and **scraped_at** for traceability.

## Related Actors

- [Google Search Scraper](https://apify.com/thescrappa/google-search-scraper)
- [Google Patents Search Scraper](https://apify.com/thescrappa/google-patents-search-scraper)

## Search terms

`Google Scholar Scraper`, `title`, `link`, `snippet`, `/google/scholar API`

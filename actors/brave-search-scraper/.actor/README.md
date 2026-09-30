# Brave Search Scraper

Search Brave results by keyword and collect organic links, snippets, and related queries.

## Data you get

- **title**: Result title or listing name.
- **link**: Result destination URL.
- **snippet**: Text excerpt shown with the result.
- **displayed_link**: Readable URL shown by the search engine.
- **position**: Position of the result on the source page.

## Use cases

- Search result monitoring
- Brand and competitor research
- Lead discovery

## How to use

Add one or more entries to **queries**. Each entry maps its **query** value to the Scrappa **query** input. Shared endpoint options can be set at the top level.

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

## Output example

This synthetic example shows the response fields and the input value attached to each result.

```json
{
  "position": 1,
  "title": "Example climate research",
  "link": "https://example.com/research",
  "redirect_link": "https://example.com/research",
  "displayed_link": "example.com/research",
  "snippet": "Synthetic research summary for fixture tests.",
  "source": "example.com",
  "input_query": "best crm software",
  "scraped_at": "2026-01-01T00:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. The Actor writes one dataset item for each result.

The Actor saves up to **maxResults** dataset items across the run.

## FAQ

### Is scraping this data legal?

Scraping rules depend on the source, the data, and how you use it. Review the applicable laws, source terms, and privacy requirements for your use case. You are responsible for your collection and use of the data.

### Are there request limits?

You can submit up to 100 batch entries per run. Set **maxResults** to cap saved rows. Scrappa API limits and source availability also apply.

### Can I use the output with integrations or the API?

Yes. Read results from the Apify dataset, use the Apify API or client libraries, or connect the dataset to your existing data workflow. Each row includes **input_query** and **scraped_at** for traceability.

## Related Actors

- [Bing Search Scraper](https://apify.com/thescrappa/bing-search-scraper)
- [Google Search Scraper](https://apify.com/thescrappa/google-search-scraper)

## Search terms

`Brave Search Scraper`, `title`, `link`, `snippet`, `/brave/search API`

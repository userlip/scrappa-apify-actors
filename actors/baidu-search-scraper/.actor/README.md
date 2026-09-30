# Baidu Search Scraper

Search Baidu results with structured titles, snippets, source details, and links.

## Data you get

- **title**: Result title or listing name.
- **link**: Result destination URL.
- **snippet**: Text excerpt shown with the result.
- **source**: Publisher, seller, or source name.
- **position**: Position of the result on the source page.

## Use cases

- China market research
- Chinese-language SEO monitoring
- Search result collection

## How to use

Add one or more entries to **queries**. Each entry maps its **query** value to the Scrappa **query** input. Shared endpoint options can be set at the top level.

```json
{
  "queries": [
    {
      "query": "apple iphone"
    }
  ],
  "limit": 10,
  "page": 1,
  "maxResults": 20,
  "maxPages": 2
}
```

## Output example

This synthetic example shows the response fields and the input value attached to each result.

```json
{
  "position": 1,
  "result_type": "organic",
  "title": "示例页面",
  "link": "https://example.com/zh",
  "displayed_link": "example.com",
  "snippet": "Synthetic Chinese search result.",
  "source": "example.com",
  "category": "web",
  "date": "2026-01-01",
  "input_query": "apple iphone",
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

Yes. Read results from the Apify dataset, use the Apify API or client libraries, or connect the dataset to your existing data workflow. Each row includes **input_query** and **scraped_at** for traceability.

## Related Actors

- [Bing Search Scraper](https://apify.com/thescrappa/bing-search-scraper)
- [Brave Search Scraper](https://apify.com/thescrappa/brave-search-scraper)

## Search terms

`Baidu Search Scraper`, `title`, `link`, `snippet`, `/baidu/search API`

# Baidu Trending Searches Scraper

Collect trending searches from Baidu trend boards by tab.

## Data you get

- **query**: Search phrase or trending term associated with the result.
- **title**: Result title or listing name.
- **position**: Position of the result on the source page.
- **link**: Result destination URL.
- **is_top**: Is top returned for this Baidu Trending Searches Scraper result.

## Use cases

- Chinese trend monitoring
- Content planning
- News and topic discovery

## How to use

Add one or more entries to **tabs**. Each entry maps its **tab** value to the Scrappa **tab** input. Shared endpoint options can be set at the top level.

```json
{
  "tabs": [
    {
      "tab": "realtime"
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
  "query": "synthetic search trend",
  "title": "Synthetic search trend",
  "link": "https://example.com/trend",
  "description": null,
  "is_top": false,
  "input_tab": "realtime",
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

Yes. Read results from the Apify dataset, use the Apify API or client libraries, or connect the dataset to your existing data workflow. Each row includes **input_tab** and **scraped_at** for traceability.

## Related Actors

- [Baidu Search Scraper](https://apify.com/thescrappa/baidu-search-scraper)
- [Google News Scraper](https://apify.com/thescrappa/google-news-scraper)

## Search terms

`Baidu Trending Searches Scraper`, `query`, `title`, `position`, `/baidu/trending API`

# Baidu Search Scraper

Collect Baidu results for multiple phrases, including rank, title, snippet, source, and destination link. Page limits bound collection for each search.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `position` | Integer | Baidu search-result rank for this page, starting at 1. |
| `result_type` | String | Baidu result category, such as an organic page or special result. |
| `title` | String | Page title displayed for the Baidu search result. |
| `link` | String | Destination URL opened by the Baidu search result. |
| `displayed_link` | String | Host or shortened address Baidu displays below the result title. |
| `snippet` | String | Text excerpt Baidu displays for the matching page. |
| `source` | String | Publisher or site name associated with the result. |
| `category` | String | Content category assigned to this Baidu result when available. |
| `date` | String | Publication or crawl date displayed with the result, when provided. |
| `input_query` | String | Chinese or multilingual search phrase submitted to Baidu. |
| `scraped_at` | String | UTC date and time when these Baidu results were collected. |

## Use cases

- China-focused SEO teams can check page positions for priority queries.
- Content strategists can compare snippets and competing domains by topic.
- Publishers can review how Baidu categorizes and dates indexed pages.

## How to use

1. Add a query object for each phrase in `queries`.
2. Set `limit` and the starting `page`; use `maxPages` to bound collection.
3. Compare position, snippet, source, and link fields in the dataset.

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

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | Array\<object\> | Yes | Baidu search phrases. Add one object per phrase. |
| `queries[].query` | string | Yes per entry | Search query. |
| `queries[].limit` | integer | No | Maximum Baidu results to request on each page. |
| `queries[].page` | integer | No | 1-based Baidu result page. |
| `limit` | integer | No | Maximum Baidu results to request on each page. |
| `page` | integer | No | 1-based Baidu result page. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "position": 1,
  "result_type": "organic",
  "title": "对象存储 OSS - 阿里云",
  "link": "https://www.aliyun.com/product/oss",
  "displayed_link": "aliyun.com/product/oss",
  "snippet": "Store and retrieve files with scalable cloud object storage from Alibaba Cloud.",
  "source": "Alibaba Cloud",
  "category": "web",
  "date": "2026-09-18",
  "input_query": "apple iphone",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~baidu-search-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### How many pages can one query collect?

Set `maxPages` up to the Actor limit. `maxResults` caps saved rows across every query.

## Related Scrappa Actors

- [Baidu Trending Searches Scraper](https://apify.com/thescrappa/baidu-trending-scraper)
- [Bing Search Scraper](https://apify.com/thescrappa/bing-search-scraper)
- [Brave Search Scraper](https://apify.com/thescrappa/brave-search-scraper)

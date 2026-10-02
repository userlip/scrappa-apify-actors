# Instagram Hashtag Search Scraper

Search Instagram for matching hashtags and collect names, public post counts, and display labels. Batch several phrases to compare how topics are represented.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `id` | String | Instagram identifier for the hashtag result. |
| `name` | String | Hashtag name without the leading number sign. |
| `media_count` | Integer | Number of public media posts Instagram associates with this hashtag. |
| `formatted_media_count` | String | Human-readable post count label returned by Instagram. |
| `search_result_subtitle` | String | Subtitle shown with the hashtag search result. |
| `input_q` | String | Hashtag or keyword submitted to find matching Instagram tags. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Social planners can compare hashtag reach before scheduling public posts.
- Researchers can map related Instagram topics from seed phrases.
- Brand teams can check public post counts for campaign terms.

## How to use

1. Add each hashtag or phrase to `queries` without the leading number sign.
2. Set `maxResults` to cap the number of hashtag rows saved.
3. Compare names and post counts across seed searches.

```json
{
  "queries": [
    {
      "q": "coffee"
    }
  ],
  "maxResults": 20
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | Array\<object\> | Yes | Hashtag names or phrases to search on Instagram. |
| `queries[].q` | string | Yes per entry | Search Query sent to the source search. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "id": "17843785412019201",
  "name": "coffeeart",
  "media_count": 278000,
  "formatted_media_count": "278K posts",
  "search_result_subtitle": "278K posts",
  "input_q": "coffee",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~instagram-hashtag-search-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Should I include the \# character?

Enter the hashtag word or phrase. The source search handles matching without requiring a leading number sign.

## Related Scrappa Actors

- [Instagram User Posts Scraper](https://apify.com/thescrappa/instagram-user-posts-cheapest-0-20-1000-results)
- [Instagram User Info \| Cheapest $0.20/1k results](https://apify.com/thescrappa/instagram-user-info-cheapest-0-20-1000-results)
- [TikTok Hashtag Posts Scraper](https://apify.com/thescrappa/tiktok-hashtag-posts-scraper)

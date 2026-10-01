# Baidu Trending Searches Scraper

Read current topics from selected Baidu trending boards. Each row includes rank, topic text, a result title and link, and the board indicator when available.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `position` | Integer | Rank of this topic on the selected Baidu trending board. |
| `query` | String | Search phrase or topic currently trending on Baidu. |
| `title` | String | Headline shown for this Baidu trend, when the board provides one. |
| `link` | String | Baidu page linked from the trending topic. |
| `description` | String or null | Short context or summary shown alongside the trend. |
| `is_top` | Boolean | True when Baidu marks the topic as a leading trend. |
| `input_tab` | String | Baidu trending board selected for this collection. |
| `scraped_at` | String | UTC date and time when this trending topic was collected. |

## Use cases

- Trend analysts can spot rising topics by board and rank.
- Editors can use current Baidu topics to plan local news and content calendars.
- Market teams can track changing interest around products and public topics.

## How to use

1. Add one Baidu trending board name to `tabs` for each board.
2. Set `maxResults` to cap topic rows saved.
3. Run the Actor and review topic, title, position, and link fields.

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

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `tabs` | Array\<object\> | Yes | Baidu trending board names to collect. |
| `tabs[].tab` | string | Yes per entry | Baidu trending board to read. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "position": 1,
  "query": "portable air conditioner",
  "title": "Portable air conditioners",
  "link": "https://www.baidu.com/s?wd=%E4%BE%BF%E6%90%BA%E5%BC%8F%E7%A9%BA%E8%B0%83",
  "description": "Search interest is rising for compact cooling options for apartments and home offices.",
  "is_top": true,
  "input_tab": "realtime",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~baidu-trending-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Which trending boards can I request?

Use a board value supported by the source, such as `realtime`; rows reflect the topics currently available.

## Related Scrappa Actors

- [Baidu Search Scraper](https://apify.com/thescrappa/baidu-search-scraper)
- [Bing Search Scraper](https://apify.com/thescrappa/bing-search-scraper)
- [Google Trends Interest Scraper](https://apify.com/thescrappa/google-trends-interest-scraper)

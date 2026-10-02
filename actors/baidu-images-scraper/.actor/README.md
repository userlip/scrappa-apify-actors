# Baidu Images Scraper

Find Baidu image results with titles, source pages, original image links, preview URLs, and dimensions. Search several phrases in one run to build visual reference datasets.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `position` | Integer | Rank of this image in Baidu results, starting at one. |
| `title` | String | Baidu result title associated with the image page. |
| `title_highlighted_words` | Array\<String\> | Words Baidu highlights in the image title. |
| `link` | String | Page URL where Baidu found or displays the image. |
| `original` | String | Direct URL reported for the original image file. |
| `thumbnail` | String | URL of the small image preview in search results. |
| `thumbnail_middle` | String | URL of the medium image preview when available. |
| `source` | String | Publisher name associated with the image result. |
| `source_domain` | String | Host name of the source page, without a protocol. |
| `original_width` | Integer | Reported width of the original image in pixels. |
| `original_height` | Integer | Reported height of the original image in pixels. |
| `file_type` | String | File extension or format reported for the original image. |
| `date` | String | Date string Baidu associates with the image result; formatting varies by source. |
| `is_gif` | Boolean | True when Baidu identifies the result as an animated GIF. |
| `input_query` | String | Baidu search phrase submitted to retrieve this image result. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Visual researchers can compare image sources and dimensions across search phrases.
- Content teams can find source pages and image URLs for topic research.
- Market analysts can track how visual results vary across Baidu queries.

## How to use

1. Add one search phrase to `queries` for each image search.
2. Set `maxResults` to cap saved image rows for the run.
3. Use the original and preview URLs to review source images.

```json
{
  "queries": [
    {
      "query": "panda"
    }
  ],
  "maxResults": 20
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | Array\<object\> | Yes | Baidu phrases to search for image results. |
| `queries[].query` | string | Yes per entry | Search Query sent to the source search. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "position": 1,
  "title": "Red panda resting in a bamboo grove",
  "title_highlighted_words": [
    "panda"
  ],
  "link": "https://wildlifejournal.org/red-panda",
  "original": "https://images.wildlife-journal.org/red-panda-full.jpg",
  "thumbnail": "https://images.wildlife-journal.org/red-panda-small.jpg",
  "thumbnail_middle": "https://images.wildlife-journal.org/red-panda-medium.jpg",
  "source": "Wildlife Journal",
  "source_domain": "wildlifejournal.org",
  "original_width": 2400,
  "original_height": 1600,
  "file_type": "jpg",
  "date": "2025-04-16",
  "is_gif": false,
  "input_query": "panda",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~baidu-images-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Does the Actor download the image files?

No. It saves the result metadata and image URLs reported by Baidu.

## Related Scrappa Actors

- [Baidu Search Scraper](https://apify.com/thescrappa/baidu-search-scraper)
- [Google Images Scraper](https://apify.com/thescrappa/google-images-scraper)
- [Google Search Scraper](https://apify.com/thescrappa/google-search)

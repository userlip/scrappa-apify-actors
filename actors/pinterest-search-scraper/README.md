# Pinterest Search Scraper

Find Pinterest pins with titles, descriptions, image links and source pages. Search a subject or style phrase to find public pins and follow their image links back to the source pages.

## What data can you extract?

Pin titles, images and source links follow the public Pinterest result page.

| Field | Type | Description |
| --- | --- | --- |
| `id` | text | source ID for the Pinterest pin, assigned by Pinterest; null when the source does not expose it. |
| `title` | text | Title of the Pinterest pin, as shown by Pinterest; null when no title is published. |
| `description` | text | Description text from Pinterest for this Pinterest pin; null when the source has no text to show. |
| `image_url` | image | Image url for this Pinterest pin on Pinterest; null when the source does not provide a URL. |
| `link` | link | Result link for this Pinterest pin on Pinterest; null when the source does not provide a URL. |
| `domain` | text | Domain shown for the Pinterest pin by Pinterest, in the format used by the source; null when it is omitted. |
| `pinner_username` | text | Pinner username shown for the Pinterest pin by Pinterest, in the format used by the source; null when it is omitted. |
| `board_name` | text | Name of the Pinterest pin, as shown by Pinterest; null when no name is published. |
| `has_video` | boolean | Whether the post includes a video; false is a reported value, while null means Pinterest provided no flag. |
| `repin_count` | number | Number of repins shown by Pinterest, as a whole number; zero is possible, and null means no count was reported. |
| `comment_count` | number | Number of comments shown by Pinterest, as a whole number; zero is possible, and null means no count was reported. |
| `like_count` | number | Number of likes shown by Pinterest, as a whole number; zero is possible, and null means no count was reported. |
| `save_count` | number | Number of saves shown by Pinterest, as a whole number; zero is possible, and null means no count was reported. |
| `request_query` | text | Search phrase passed to Pinterest. This input value is copied into the output row; null when it was not supplied. |
| `request_limit` | number | Maximum result count passed to Pinterest; A whole-number maximum count. This input value is copied into the output row; null when it was not supplied. |
| `request_bookmark` | text | Pagination bookmark passed to Pinterest. This input value is copied into the output row; null when it was not supplied. |
| `count` | number | Number of results shown by Pinterest, as a whole number; zero is possible, and null means no count was reported. |
| `results_count` | number | Number of results shown by Pinterest, as a whole number; zero is possible, and null means no count was reported. |
| `nextBookmark` | text | Next bookmark shown for the Pinterest pin by Pinterest, in the format used by the source; null when it is omitted. |

## Use cases

- Designers can collect pin images and linked domains for a visual mood board.
- Content teams can review Pinterest results before planning a collection.
- Researchers can compare public pins and linked pages across a topic.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `queries` and use the identifier or URL format required by Pinterest.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "queries": [
    "home decor",
    "kitchen ideas"
  ],
  "limit": 5
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | array of string | No | Pinterest keyword searches to run in one Apify Actor execution. |
| `query` | string | No | Compatibility field for one Pinterest keyword search. Prefer Search Queries for batching. |
| `limit` | integer | No | Maximum Pinterest pins to request from Scrappa for each query. Pinterest may return fewer pins than requested. Constraints: minimum 1; maximum 250. |
| `bookmark` | string | No | Optional Pinterest pagination bookmark returned as nextBookmark by a previous run. The same bookmark is applied to each query in this run. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Balcony Herb Garden Ideas",
  "description": "Ideas for growing basil, mint and parsley in bright window boxes and compact balcony planters.",
  "link": "https://www.example.com/gardening/balcony-herbs",
  "id": "839472610948372615",
  "image_url": "https://images.example.com/gardening/balcony-herbs.jpg",
  "domain": "example.com",
  "pinner_username": "harborlight_gardens",
  "board_name": "Small Space Gardening"
}
```

## Pricing

**Current live price:** $0.20 per 1,000 results.

Each saved source match counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~pinterest-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I find Pinterest pins for a visual topic?

Enter a specific phrase in the search input. Pinterest may return titles, descriptions, image links and source pages for matching public pins.

## Related Scrappa Actors

- [Instagram Post Info | Cheapest $0.20/1k results](https://apify.com/thescrappa/instagram-post-info-cheapest-0-20-1000-results)
- [Instagram User Info | Cheapest $0.20/1k results](https://apify.com/thescrappa/instagram-user-info-cheapest-0-20-1000-results)
- [Instagram User Posts Scraper](https://apify.com/thescrappa/instagram-user-posts-cheapest-0-20-1000-results)
- [LinkedIn Post Scraper](https://apify.com/thescrappa/linkedin-post-scraper)
- [TikTok Hashtag Details Scraper](https://apify.com/thescrappa/tiktok-challenge-details-scraper)

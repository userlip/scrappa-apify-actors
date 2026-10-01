# LinkedIn Post Scraper

Read a public LinkedIn post with its text, author, publication date and reaction count. Paste a public LinkedIn post URL to retrieve its text and engagement details.

## What data can you extract?

Profile, company and post details reflect public LinkedIn pages; the source may omit optional fields.

| Field | Type | Description |
| --- | --- | --- |
| `title` | text | Title of the LinkedIn post, as shown by LinkedIn; null when no title is published. |
| `author_name` | text | Name of the LinkedIn member who published the post; null when no author name is available. |
| `date_published` | text | Date the article was published shown by LinkedIn, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `reactions_total` | number | Number of reactions shown by LinkedIn, as a whole number; zero is possible, and null means no count was reported. |
| `comments_count` | number | Number of comments shown by LinkedIn, as a whole number; zero is possible, and null means no count was reported. |

## Use cases

- Teams can review source records before a follow-up decision.
- Researchers can compare available records across targets or runs.
- Analysts can use source links to maintain a focused dataset.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `url` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "url": "https://www.linkedin.com/posts/microsoft_worktrendindex-activity-7457369463198437376-3F4k"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `url` | string | Yes | URL of the LinkedIn post or article to scrape (e.g., https://linkedin.com/pulse/... or https://linkedin.com/posts/...) |
| `use_cache` | boolean | No | Use cached results if available to reduce costs. When disabled, the actor omits the cache flag because Scrappa does not accept use_cache=0. |
| `maximum_cache_age` | integer | No | Maximum age of cached results in seconds. Must be at least 1. Constraints: minimum 1. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "A practical guide to independent neighborhood shops",
  "author_name": "Morgan Lee",
  "date_published": "2026-09-25",
  "reactions_total": 1380,
  "comments_count": 27
}
```

## Pricing

**Current live price:** $0.30 per 1,000 results.

Each saved dataset record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~linkedin-post-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can LinkedIn Post Scraper read a post from its URL?

Yes. Paste the public post URL in `url`. Login-gated or removed posts may not expose their text and engagement details.

## Related Scrappa Actors

- [LinkedIn Company Scraper](https://apify.com/thescrappa/linkedin-company-scraper)
- [LinkedIn Job Details Scraper](https://apify.com/thescrappa/linkedin-job-details-scraper)
- [LinkedIn Jobs Search Scraper](https://apify.com/thescrappa/linkedin-jobs-search-scraper)
- [LinkedIn Profile Scraper](https://apify.com/thescrappa/linkedin-profile-scraper)
- [LinkedIn Search Scraper](https://apify.com/thescrappa/linkedin-search-scraper)

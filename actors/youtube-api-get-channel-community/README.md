# YouTube API Get Channel Community

Fetch community posts for a YouTube channel by channel ID. The Actor calls the Scrappa YouTube channel community endpoint and saves returned posts to the default Apify dataset, subject to the run's pay-per-event spending limit.

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | YouTube channel ID. |
| `continuation` | string | No | Pagination token returned by a previous run. |

## Example Input

```json
{
  "id": "UCJZv4d5rbIKd4QHMPkcABCw"
}
```

## Output

Each saved community post is stored as one dataset item. Fields depend on the current Scrappa YouTube response, and typically include the post ID, text, published time, like count, comment count, and attachments such as images, videos, polls, or quizzes.

## Pagination

When the Scrappa API returns a continuation token, the Actor logs it. Use that value as the `continuation` input to fetch the next page of community posts.

## Pricing

$0.30 per 1,000 results. No additional API keys required.
On pay-per-event runs, the Apify spending limit can reduce the number of fetched posts saved; the Actor keeps the original order and saves only the prefix that fits the remaining dataset-item budget.

## Support

For issues or questions, contact us through Apify.

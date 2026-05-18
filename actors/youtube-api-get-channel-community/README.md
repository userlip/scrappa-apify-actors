# YouTube API Get Channel Community

Fetch community posts for a YouTube channel by channel ID. The Actor calls the Scrappa YouTube channel community endpoint and saves each returned post to the default Apify dataset.

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

The Actor stores one dataset item per community post. Fields depend on the current Scrappa YouTube response, and typically include the post ID, text, published time, like count, comment count, and attachments such as images, videos, polls, or quizzes.

## Pagination

When the Scrappa API returns a continuation token, the Actor logs it. Use that value as the `continuation` input to fetch the next page of community posts.

## Pricing

$0.30 per 1,000 results. No additional API keys required.

## Support

For issues or questions, contact us through Apify.

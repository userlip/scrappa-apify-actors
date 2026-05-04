# YouTube API Video Comments

Fetch comments and reply threads for a YouTube video by video ID. The Actor supports YouTube comment sort order and continuation tokens for pagination.

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | YouTube video ID. |
| `sort` | array | No | Sort order selected in Apify. One of `TOP_COMMENTS` or `NEWEST_FIRST`. |
| `continuation` | string | No | Pagination token for the next comments page. |

## Example Input

```json
{
  "id": "dQw4w9WgXcQ",
  "sort": ["TOP_COMMENTS"]
}
```

## Continue to the Next Page

```json
{
  "id": "dQw4w9WgXcQ",
  "sort": ["NEWEST_FIRST"],
  "continuation": "PASTE_CONTINUATION_TOKEN_FROM_PREVIOUS_RUN"
}
```

When the API returns a next-page token, the Actor logs it as `Continuation token available for next page: ...`.

## Output

The Actor stores one dataset item per top-level comment returned by the API. Fields typically include the comment text, author metadata, like counts, publish time, reply counts, and embedded replies when available.

## Pricing

$0.30 per 1,000 results. No additional API keys required.

## Support

For issues or questions, contact us through Apify.

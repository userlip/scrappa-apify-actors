# YouTube API Related Videos

Fetch related YouTube videos for a single video ID. The Actor calls Scrappa's YouTube related videos endpoint and saves each related video as a separate Apify dataset item.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `id` | string | Yes | YouTube video ID. |
| `continuation` | string | No | Continuation token from a previous response for fetching the next page of related videos. |

Example:

```json
{
  "id": "dQw4w9WgXcQ",
  "continuation": "optional-next-page-token"
}
```

## Output

The Actor stores one dataset item per related video returned by the API. Fields depend on the current Scrappa YouTube response, and typically include the video ID, title, URL, thumbnail, duration, view count, publish metadata, and channel metadata.

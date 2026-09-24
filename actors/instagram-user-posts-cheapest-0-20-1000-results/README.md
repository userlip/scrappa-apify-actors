# Instagram User Posts

Fetch recent public Instagram posts and reels by username, including captions, media, engagement metrics, permalinks, and pagination cursors.

## Input

```json
{
  "username": "natgeo"
}
```

Use the username without the `@` symbol. The actor also accepts usernames with `@` and normalizes them before sending the request.

To fetch the next page, pass the `next_max_id` value from the previous run's `OUTPUT` key-value store record as `max_id`:

```json
{
  "username": "natgeo",
  "max_id": "QVFDcF..."
}
```

## Output

Each run pushes one dataset item per returned post. If Scrappa returns zero posts, the dataset remains empty. The full Scrappa API response is also saved to the default key-value store as `OUTPUT`.

For `PAY_PER_EVENT` runs, dataset writes respect the run's `maxTotalChargeUsd` limit. The Actor checks current event charges and saves only as many posts as the remaining budget allows. Other pricing models save all returned posts. The `OUTPUT` record retains the full upstream response.

## Runtime

This Actor is implemented in Rust. The Scrappa request deadline is 90 seconds; the Apify run timeout is 120 seconds.

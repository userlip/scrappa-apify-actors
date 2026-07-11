# TikTok Hashtag & Challenge Details Scraper

Resolve a known TikTok hashtag or stable challenge ID into canonical metadata through Scrappa. This Actor is for authoritative challenge resolution, not keyword discovery; use TikTok Challenge Search Scraper when you need to find candidate hashtags first.

Submit up to 100 distinct names and IDs in one run. Each successful lookup produces one dataset row; unavailable or renamed challenges are reported in the compact `OUTPUT` summary without failing other lookups.

## Inputs

| Field | Type | Description |
| --- | --- | --- |
| `challenge_names` | string list | Preferred batch input. Leading `#` is optional. |
| `challenge_ids` | string list | Preferred batch input for stable numeric IDs. |
| `challenge_name` | string | Backward-compatible single name; combined with batch fields. |
| `challenge_id` | string or safe integer | Backward-compatible single ID; combined with batch fields. |

Names are deduplicated case-insensitively and IDs exactly. Names and IDs remain separate lookups. The combined limit is 100 entities.

### Campaign research

```json
{ "challenge_names": ["booktok", "fitness", "summerreads"] }
```

### View and user-count analysis

```json
{ "challenge_ids": ["1622962893630470"], "challenge_names": ["booktok"] }
```

### Resolve a name before requesting challenge posts

```json
{ "challenge_name": "#booktok" }
```

Use the returned canonical `challenge_id` with the TikTok Hashtag Posts Scraper when you need posts for a resolved challenge.

## Output

Successful dataset items preserve Scrappa/TikTok raw fields and add stable normalized fields including `challenge_id`, `challenge_name`, `description`, `user_count`, `view_count`, `video_count`, `cover`, request provenance, and `retrieved_at`. Counters are observations at that timestamp and can change.

`OUTPUT` contains one compact batch summary with requested, attempted, saved, failed, charge-limit, and per-entity outcome information. The default dataset is the primary result channel.

## Pricing

Publish with Apify `PAY_PER_EVENT` pricing using the `challenge-detail-result` event at `$0.00025` per successfully saved item. Verify active paid pricing, or the earliest scheduled paid activation, through Apify before making the Actor public.

For higher-volume usage or direct API access, use [Scrappa](https://scrappa.co).

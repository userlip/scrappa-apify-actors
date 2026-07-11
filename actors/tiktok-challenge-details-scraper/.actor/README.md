# TikTok Hashtag & Challenge Details Scraper

This Actor resolves known TikTok hashtag names and stable challenge IDs into canonical challenge metadata. Batch up to 100 combined names and IDs in one run; every successful detail record is a dataset item.

It is distinct from challenge search: use search to discover candidates, then use this Actor for authoritative name/ID resolution, campaign research, user/view-count snapshots, or before requesting challenge posts.

```json
{
  "challenge_names": ["booktok"],
  "challenge_ids": ["1622962893630470"]
}
```

The `challenge-detail-result` pay-per-event charge applies only to successfully saved records. Failures remain in the compact `OUTPUT` summary and do not charge.

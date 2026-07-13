# Instagram User Info

Fetch up to 100 Instagram profiles per Apify run through Scrappa. Use the `usernames` array for cost-efficient batches; the legacy `username` field remains supported. Each processed username produces one dataset item.

Get public Instagram profile data by username, including biography, follower counts, verification status, profile picture, and related profile fields.

## Input

```json
{
  "usernames": ["natgeo", "instagram"]
}
```

Use `usernames` for batches of up to 100 profiles. The legacy `username` string is still accepted. Handles can include `@`; the Actor removes it before sending each request.

## Output

Each processed username pushes one profile object to the default dataset. Failed lookups produce a failure item for that username so the batch remains auditable.

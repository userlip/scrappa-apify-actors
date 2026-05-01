# Instagram User Info Scraper

Extract public Instagram profile information by username. This Actor is built for profile lookups, creator research, lead enrichment, competitor monitoring, and quick checks of public Instagram account metadata.

No Instagram login, cookies, proxy setup, or browser session is required. Provide a username and the Actor returns the available public profile fields to an Apify dataset. The Actor uses a Scrappa API key configured as the `SCRAPPA_API_KEY` environment variable; set this secret if you fork or self-deploy the Actor.

## What It Does

- Public profile identity fields such as username, full name, biography, external URL, profile picture, and category.
- Audience and activity counts such as followers, following, and media count.
- Account flags such as verified, private, business, professional, and related public profile indicators when Instagram returns them.
- Raw response fields from the upstream profile lookup, flattened into one dataset item for easier export and filtering.

Private accounts can still return public metadata that is visible without logging in, but this Actor does not bypass privacy restrictions or access private posts.

## Input

Use one Instagram username per run.

### Input Fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `username` | String | Yes | Instagram username to look up. Use a handle such as `natgeo`; an optional leading `@` is normalized automatically. |

## Example Input

```json
{
  "username": "natgeo"
}
```

You can enter the username with or without the `@` symbol. For example, `natgeo` and `@natgeo` are both accepted.

## Output

Each run saves one profile object to the default Apify dataset. The exact fields can vary depending on what Instagram returns for the profile, but common fields include:

| Field | Description |
| --- | --- |
| `username` | Instagram handle. |
| `full_name` | Display name shown on the profile. |
| `biography` | Public profile bio text. |
| `external_url` | Website URL shown on the profile, when available. |
| `profile_pic_url` | Public profile picture URL, when available. |
| `follower_count` | Number of followers. |
| `following_count` | Number of accounts followed. |
| `media_count` | Number of posts or media items reported for the account. |
| `is_verified` | Whether the account is verified. |
| `is_private` | Whether the account is private. |
| `is_business_account` | Whether Instagram marks the profile as a business account, when available. |
| `is_professional_account` | Whether Instagram marks the profile as a professional account, when available. |
| `category_name` | Public category label, when available. |

## Example Output

```json
{
  "username": "natgeo",
  "full_name": "National Geographic",
  "biography": "Experience the world through the eyes of National Geographic photographers.",
  "external_url": "https://www.nationalgeographic.com/",
  "follower_count": 280000000,
  "following_count": 170,
  "media_count": 30000,
  "is_verified": true,
  "is_private": false,
  "profile_pic_url": "https://..."
}
```

## Exporting results

Results are stored in the Actor's default dataset. From Apify, you can export the dataset as:

- JSON
- CSV
- Excel
- XML
- RSS
- HTML table

Use JSON for full nested/raw fields, or CSV and Excel when you want spreadsheet-friendly columns for profile research, enrichment, or reporting.

## Authentication

This Actor does not require an Instagram account. It does not ask for Instagram credentials, session cookies, or two-factor authentication codes.

That makes it simple to run in Apify tasks, schedules, and API workflows. It also means the Actor only returns public information available through the profile lookup and does not unlock private account content.

## Common Use Cases

- Enrich a list of Instagram handles with follower counts and bios.
- Check whether a public account is verified, private, or business-related.
- Monitor creator, brand, competitor, or publisher profile metadata.
- Export profile records into a CRM, spreadsheet, or data warehouse.
- Validate Instagram usernames before running larger data workflows.

## Recommended Workflow

1. Prepare a list of Instagram usernames from your CRM, spreadsheet, research workflow, or another Apify Actor.
2. Run this Actor once per username to fetch the public profile record.
3. Export the dataset as JSON when you need full raw fields, or CSV/Excel when you need spreadsheet-friendly profile columns.
4. Join the exported profile data with your existing leads, creator lists, competitor trackers, or enrichment pipeline.

## Tips For Better Results

- Use exact Instagram usernames instead of display names or profile URLs.
- Keep both `username` and any exported account ID fields in downstream systems so you can deduplicate profile records later.
- Re-run important profiles periodically if you monitor follower count, biography, verification, or business-account changes.
- For high-volume enrichment across many usernames, use Apify tasks or API calls to schedule repeat runs.

## Pricing

$0.20 per 1,000 results. No Instagram login required. Requires `SCRAPPA_API_KEY` in the Actor environment.

## Notes and limits

- Run one username per Actor run.
- Usernames are normalized before lookup, so a leading `@` is removed automatically.
- Availability of some fields depends on the public data returned for that profile.
- If Instagram or the upstream profile source does not expose a field for a profile, it may be missing or null in the dataset item.

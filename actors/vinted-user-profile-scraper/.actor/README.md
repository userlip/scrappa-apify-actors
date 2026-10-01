# Vinted User Profile Scraper

Look up a Vinted seller profile with username, location, feedback and active item count. Paste the seller or user ID shown in the Input section to retrieve its public record.

## What data can you extract?

Seller feedback and active item counts reflect the public Vinted profile at lookup time.

| Field | Type | Description |
| --- | --- | --- |
| `id` | text | source ID for the seller profile, assigned by Vinted; null when the source does not expose it. |
| `login` | text | Vinted seller username shown for the seller profile by Vinted, in the format used by the source; null when it is omitted. |
| `country_code` | text | Country code shown for the seller profile by Vinted; null when Vinted does not provide the value. |
| `city` | text | City shown for the seller profile by Vinted; null when Vinted does not provide the value. |
| `feedback_count` | number | Number of seller-feedbacks shown by Vinted, as a whole number; zero is possible, and null means no count was reported. |
| `feedback_reputation` | number | Feedback reputation shown for the seller profile by Vinted, in the format used by the source; null when it is omitted. |
| `positive_feedback_count` | number | Number of positive-feedbacks shown by Vinted, as a whole number; zero is possible, and null means no count was reported. |
| `neutral_feedback_count` | number | Number of neutral-feedbacks shown by Vinted, as a whole number; zero is possible, and null means no count was reported. |
| `negative_feedback_count` | number | Number of negative-feedbacks shown by Vinted, as a whole number; zero is possible, and null means no count was reported. |
| `bundle_discount_enabled` | boolean | Whether the seller offers bundle discounts; false is a reported value, while null means Vinted provided no flag. |
| `bundle_discounts` | array | Seller discount tiers with minimum item quantity and discount rate from Vinted; an empty list when no entries are available. |
| `item_count` | number | Number of items shown by Vinted, as a whole number; zero is possible, and null means no count was reported. |
| `total_items_count` | number | Number of items shown by Vinted, as a whole number; zero is possible, and null means no count was reported. |
| `followers_count` | number | Number of followers shown by Vinted, as a whole number; zero is possible, and null means no count was reported. |
| `following_count` | number | Number of followings shown by Vinted, as a whole number; zero is possible, and null means no count was reported. |
| `last_activity` | text | Last activity shown for the seller profile by Vinted, in the format used by the source; null when it is omitted. |
| `is_email_verified` | boolean | Whether the email is marked as verified; false is a reported value, while null means Vinted provided no flag. |
| `is_facebook_verified` | boolean | Whether Facebook verification is reported; false is a reported value, while null means Vinted provided no flag. |
| `is_google_verified` | boolean | Whether Google verification is reported; false is a reported value, while null means Vinted provided no flag. |
| `business` | boolean | Whether the seller is a business account; false is a reported value, while null means Vinted provided no flag. |
| `is_on_holiday` | boolean | Whether the seller is marked as away; false is a reported value, while null means Vinted provided no flag. |
| `is_account_banned` | boolean | Whether the account is marked as banned; false is a reported value, while null means Vinted provided no flag. |
| `profile_url` | link | Profile url for this seller profile on Vinted; null when the source does not provide a URL. |
| `request_user_id` | text | Source user id passed to Vinted. This input value is copied into the output row; null when it was not supplied. |
| `request_country` | text | Country code or country name passed to Vinted. This input value is copied into the output row; null when it was not supplied. |
| `request_index` | number | Zero-based position of this request in the submitted Vinted input batch; null for a single-item lookup. |
| `request_success` | boolean | Lookup success flag passed to Vinted. This input value is copied into the output row; null when it was not supplied. |
| `scrappa_duration_ms` | number | Response time for this Vinted lookup, measured in milliseconds; null when no timing value was recorded. |
| `scrappa_scraped_at` | date | Time the page was retrieved shown by Vinted, in ISO 8601 date and time; null if the source omits the date. |

## Use cases

- Resellers can compare listings by title, price, condition and location.
- Marketplace teams can monitor inventory for a brand or category.
- Catalog operators can collect source-linked records for product research.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Provide a public Vinted identifier or URL in `user_ids`; optional fields control the lookup where supported.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "user_ids": [
    "255914028"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `user_id` | string | No | Single Vinted user ID. Use user_ids for batches; both fields can be combined. |
| `user_ids` | array/string | No | Batch of Vinted user IDs as an array or comma-separated string. The Actor validates numeric IDs and enforces a maximum of 100 unique IDs per run. |
| `country` | string | No | Vinted country market. Defaults to FR. Constraints: allowed values: FR, DE, ES, IT, NL, BE, AT, PL, CZ, LT, LU, SK, HU, RO, PT, SE, DK, FI, US. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "bundle_discounts": [
    {
      "fraction": "0.05",
      "minimum_quantity": 2
    },
    {
      "fraction": "0.10",
      "minimum_quantity": 3
    }
  ],
  "id": "3482719082",
  "login": "cedarandpine",
  "country_code": "US",
  "city": "Seattle",
  "feedback_count": 18,
  "feedback_reputation": 7.3,
  "positive_feedback_count": 18
}
```

## Pricing

**Current live price:** $0.50 per 1,000 results.

Each saved listing or profile record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~vinted-user-profile-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Where do I find a Vinted user ID?

Use a public seller profile or item page to identify the numeric user ID, then submit it through `user_id` or `user_ids`.

## Related Scrappa Actors

- [Kleinanzeigen Listing Details Scraper](https://apify.com/thescrappa/kleinanzeigen-listing-details-scraper)
- [Kleinanzeigen Search Scraper](https://apify.com/thescrappa/kleinanzeigen-search-scraper)
- [Vinted Item Details Scraper](https://apify.com/thescrappa/vinted-item-details-scraper)
- [Vinted Search Scraper](https://apify.com/thescrappa/vinted-search-scraper)
- [Vinted User Items Scraper](https://apify.com/thescrappa/vinted-user-items-scraper)

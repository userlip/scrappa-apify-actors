# Jameda Doctor Details Scraper

Review a Jameda doctor profile with name, specialty, ratings and practice details. Submit one or more public doctor profile URLs to collect specialty and practice information together.

## What data can you extract?

Doctor details and patient reviews follow public Jameda pages; profiles and reviews can omit optional details.

| Field | Type | Description |
| --- | --- | --- |
| `doctor_name` | text | Doctor name shown for the doctor profile by Jameda, in the format used by the source; null when it is omitted. |
| `title` | text | Title of the doctor profile, as shown by Jameda; null when no title is published. |
| `specialty` | text | Medical specialty shown for the doctor profile by Jameda; null when Jameda does not provide the value. |
| `rating` | text | Rating for this doctor profile, on the rating scale shown by Jameda; null when no score is shown. |
| `rating_number` | number | Rating number for this doctor profile, on the rating scale shown by Jameda; null when no score is shown. |
| `review_count` | text | Number of reviews shown by Jameda, as a whole number; zero is possible, and null means no count was reported. |
| `review_count_number` | number | Number of reviews shown by Jameda, as a whole number; zero is possible, and null means no count was reported. |
| `clinic_name` | text | Clinic name shown for the doctor profile by Jameda, in the format used by the source; null when it is omitted. |
| `phone` | text | Public phone shown by Jameda; null when the profile or listing does not publish contact details. |
| `website_url` | link | Website url for this doctor profile on Jameda; null when the source does not provide a URL. |
| `address` | text | Address shown for the doctor profile by Jameda, in the format used by the source; null when it is omitted. |
| `city` | text | City shown for the doctor profile by Jameda; null when Jameda does not provide the value. |
| `postal_code` | text | Postal code shown for the doctor profile by Jameda, in the format used by the source; null when it is omitted. |
| `latitude` | number | Latitude for this doctor profile on Jameda, in decimal degrees; null when the source provides no coordinates. |
| `longitude` | number | Longitude for this doctor profile on Jameda, in decimal degrees; null when the source provides no coordinates. |
| `services_count` | number | Number of services shown by Jameda, as a whole number; zero is possible, and null means no count was reported. |
| `focus_areas_count` | number | Number of focus-areas shown by Jameda, as a whole number; zero is possible, and null means no count was reported. |
| `conditions_count` | number | Number of conditions shown by Jameda, as a whole number; zero is possible, and null means no count was reported. |
| `languages_count` | number | Number of languages shown by Jameda, as a whole number; zero is possible, and null means no count was reported. |
| `doctor_url` | link | Doctor url for this doctor profile on Jameda; null when the source does not provide a URL. |
| `requested_doctor_url` | link | Requested doctor url for this doctor profile on Jameda; null when the source does not provide a URL. |
| `response_source` | text | Source used for the response shown for the doctor profile by Jameda, in the format used by the source; null when it is omitted. |
| `scraped_at` | date | Time the page was retrieved shown by Jameda, in ISO 8601 date and time; null if the source omits the date. |

## Use cases

- Practice managers can review public provider profiles and patient feedback.
- Patients can compare doctor ratings and written feedback for a specialty.
- Healthcare researchers can summarize public review themes across practices.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `doctorUrls` and use the identifier or URL format required by Jameda.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "doctorUrls": [
    "https://www.jameda.de/taylor-morgan/zahnarzt/seattle"
  ],
  "doctorUrl": "https://www.jameda.de/taylor-morgan/zahnarzt/seattle"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `doctorUrls` | array of string | No | Recommended. Process many Jameda doctor profile URLs in one Apify run. Each successful doctor URL produces one dataset item. Constraints: maximum 100 items. |
| `doctorUrl` | string | No | Backward-compatible single Jameda doctor profile URL or path. Prefer Doctor URLs for normal usage, especially when enriching more than one profile. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "A practical guide to independent neighborhood shops",
  "rating": "4.7/5",
  "review_count": "184",
  "doctor_name": "Taylor Morgan",
  "specialty": "Family medicine",
  "rating_number": 4.7,
  "review_count_number": 184,
  "clinic_name": "Clinic name for the doctor profile on Jameda"
}
```

## Pricing

**Current live price:** $0.30 per 1,000 results.

Each completed profile or detail lookup counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~jameda-doctor-details-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Jameda Doctor Details process a list of doctor profiles?

Yes. Submit doctor profile URLs in `doctorUrls` or use `doctorUrl` for a single profile. Details can be missing if a profile is no longer public.

## Related Scrappa Actors

- [Jameda Search Scraper](https://apify.com/thescrappa/jameda-search-scraper)
- [Jameda Reviews Scraper](https://apify.com/thescrappa/jameda-reviews-scraper)
- [Google Maps Search Scraper](https://apify.com/thescrappa/google-maps-search-scraper)
- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)

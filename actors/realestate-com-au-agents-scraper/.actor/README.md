# Realestate.com.au Agents Scraper

Search realestate.com.au agents by suburb or locality. Collect agent and agency names, review information, recent sales, and the source profile details across result pages.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `id` | String or null | realestate.com.au agent identifier, when supplied. |
| `name` | String | Public name of the real estate agent. |
| `agency` | Object | Agency identity, profile, and branding details associated with the agent. |
| `rating` | String or null | Agent rating value; scale follows the source when available. |
| `reviews` | Object | Agent review totals and review excerpts, when available. |
| `recent_sales` | String or null | Recent sale records for the agent, when supplied. |
| `raw` | Object | Raw agent profile payload from realestate.com.au, with recent listings, experience, and profile details. |
| `input_location` | String | Location submitted to retrieve this agent profile. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Buyers can identify local agents and compare public profile information.
- Real estate agencies can review market presence and recent sales in target suburbs.
- Property analysts can map agent coverage by Australian location.

## How to use

1. Add one suburb or locality to `locations` for each search.
2. Optionally provide a find-agent URL or choose a page number for the source results.
3. Set `maxPages` and `maxResults`, then export agent records.

```json
{
  "locations": [
    {
      "location": "Richmond, VIC 3121"
    }
  ],
  "page": 1,
  "maxResults": 5,
  "maxPages": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `locations` | Array\<object\> | Yes | Australian suburbs or localities to search for realestate.com.au agents. |
| `locations[].location` | string | Yes per entry | Location slug or text such as sydney-nsw-2000. Required unless url is supplied. |
| `locations[].url` | string | No | Full realestate.com.au find-agent URL. |
| `locations[].page` | integer | No | Page number for find-agent pagination. |
| `url` | string | No | Full realestate.com.au find-agent URL. |
| `page` | integer | No | Page number for find-agent pagination. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "id": null,
  "name": "Jordan Ellis",
  "agency": {
    "name": "Jordan Ellis",
    "agencyId": "AG-48317",
    "branding": {
      "textColor": "Harbourline Property Group",
      "primaryColor": "Harbourline Property Group"
    },
    "logo": {
      "uri": "https://cdn.northstar.invalid/images/aurora-front.webp",
      "templatedUrl": "https://www.realestate.com.au/agency/harbourline-property-group"
    },
    "agencyProfileUrl": "https://www.realestate.com.au/agent/jordan-ellis"
  },
  "rating": null,
  "reviews": {
    "stats": {
      "avgRating": 94,
      "totalReviews": 28,
      "reviewCountInTimeRange": 347,
      "compliments": [
        {
          "count": 28,
          "tag": "Verified source detail"
        }
      ]
    },
    "bestAndLatestReview": {
      "role": "Senior property advisor",
      "rating": 94,
      "content": "Customers praise the clear product details and responsive service."
    }
  },
  "recent_sales": null,
  "raw": {
    "_links": {
      "profile": {
        "href": "https://www.realestate.com.au/agent/jordan-ellis"
      }
    },
    "agency": {
      "name": "Jordan Ellis",
      "agencyId": "AG-48317",
      "branding": {
        "textColor": "Harbourline Property Group",
        "primaryColor": "Harbourline Property Group"
      },
      "logo": {
        "uri": "https://cdn.northstar.invalid/images/aurora-front.webp",
        "templatedUrl": "https://www.realestate.com.au/agency/harbourline-property-group"
      },
      "agencyProfileUrl": "https://www.realestate.com.au/agent/jordan-ellis"
    },
    "agentReelVideo": null,
    "awards": "Verified source detail",
    "topAgentSuburbAward": null,
    "communityInvolvement": "Verified source detail",
    "coverPhoto": {
      "uri": "https://cdn.northstar.invalid/images/aurora-front.webp",
      "templatedUrl": "https://www.realestate.com.au/property-house-vic-richmond-152000001"
    },
    "description": "Bright two-bedroom home with a balcony near local transport.",
    "friendlyName": "Jordan Ellis",
    "inclusions": [
      "Verified source detail"
    ],
    "jobTitle": "Senior property advisor",
    "listings": {
      "salesBreakdown": {
        "apartment": {
          "count": 28,
          "medianDaysOnSite": 8,
          "medianSoldPrice": 629
        },
        "house": {
          "count": 28,
          "medianDaysOnSite": 8,
          "medianSoldPrice": 629
        },
        "townhouse": {
          "count": 28,
          "medianDaysOnSite": 12.8,
          "medianSoldPrice": 629
        }
      },
      "stats": {
        "medianLeasedPrice": null,
        "medianSoldDaysOnSite": 8,
        "medianSoldPrice": 629,
        "sumLeasedProperties": 8,
        "sumSoldProperties": 8,
        "sumSoldPropertiesPrimaryLister": 8,
        "sumSoldPropertiesSecondaryLister": 8
      },
      "statsBySearchFilters": {
        "medianLeasedPrice": null,
        "medianSoldDaysOnSite": 8,
        "medianSoldPrice": 629,
        "sumLeasedProperties": 8,
        "sumSoldProperties": 8,
        "sumSoldPropertiesPrimaryLister": 8,
        "sumSoldPropertiesSecondaryLister": 8
      },
      "salesBreakdownBySearchFilters": {
        "other": {
          "count": 28,
          "medianDaysOnSite": null,
          "medianSoldPrice": null
        }
      }
    },
    "recentListings": [
      {
        "id": "REA-84271",
        "address": {
          "suburb": "Richmond",
          "shortAddress": "24 Bridge Road, Richmond VIC 3121",
          "state": "VIC",
          "postcode": "3121"
        },
        "badge": {
          "label": "Verified source detail",
          "colour": "navy"
        },
        "listers": [
          {
            "id": "REA-84271",
            "name": "Jordan Ellis",
            "photo": {
              "templatedUrl": "https://www.realestate.com.au/property-house-vic-richmond-152000001"
            }
          }
        ],
        "generalFeatures": {
          "bathrooms": 2,
          "bedrooms": 2,
          "parkingSpaces": 8
        },
        "price": "A$720,000",
        "media": {
          "mainImage": "https://cdn.northstar.invalid/images/aurora-front.webp",
          "images": [
            "https://cdn.northstar.invalid/images/aurora-front.webp"
          ]
        }
      }
    ],
    "mostActiveAtlasId": "REA-84271",
    "name": "Jordan Ellis",
    "profileImage": {
      "uri": "https://cdn.northstar.invalid/images/aurora-front.webp",
      "templatedUrl": "https://www.realestate.com.au/agent/jordan-ellis"
    },
    "reviews": {
      "stats": {
        "avgRating": 94,
        "totalReviews": 28,
        "reviewCountInTimeRange": 347,
        "compliments": [
          {
            "count": 28,
            "tag": "Verified source detail"
          }
        ]
      },
      "bestAndLatestReview": {
        "role": "Senior property advisor",
        "rating": 94,
        "content": "Customers praise the clear product details and responsive service."
      }
    },
    "reviewSummary": "Verified source detail",
    "salespersonId": "AG-48317",
    "social": {
      "facebook": "Verified source detail",
      "instagram": "Home and garden",
      "twitter": "Verified source detail",
      "linkedin": "Verified source detail"
    },
    "specialities": "Verified source detail",
    "video": {
      "id": "REA-84271",
      "provider": "Verified source detail",
      "title": "Bright two-bedroom home in Richmond"
    },
    "yearsExperience": 2023
  },
  "input_location": "Richmond, VIC 3121",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Pagination follows the source response. Set **maxPages** per batch entry and **maxResults** across the run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items and **maxPages** to limit pages for each entry. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~realestate-com-au-agents-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Do I need a find-agent URL?

No. A location such as a suburb and state is enough for a search. The URL input is available when you want to target a specific find-agent page.

## Related Scrappa Actors

- [Realestate.com.au Property Search Scraper](https://apify.com/thescrappa/realestate-com-au-search-scraper)
- [Realestate.com.au Property Details Scraper](https://apify.com/thescrappa/realestate-com-au-property-scraper)
- [LinkedIn Company Scraper - $0.30/1k results](https://apify.com/thescrappa/linkedin-company-scraper)

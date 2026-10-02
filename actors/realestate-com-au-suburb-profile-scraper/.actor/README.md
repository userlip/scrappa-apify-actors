# Realestate.com.au Suburb Profile Scraper

Retrieve a realestate.com.au suburb profile by URL. The profile contains market trends, median prices, rental yields, nearby areas, and the source market data.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `success` | Boolean | True when the realestate.com.au request returned a successful response. |
| `data` | Object | realestate.com.au response payload for this suburb profile, including `url`, `profile`, `raw` and related source fields. |
| `profile_url` | String | Public realestate.com.au URL for the suburb profile or linked resource. |
| `input_url` | String | Url submitted to retrieve this suburb profile. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Investors can compare market trends and median prices across suburbs.
- Buyers can review local sales and rental indicators before choosing an area.
- Housing analysts can archive suburb profiles for market monitoring.

## How to use

1. Add a full realestate.com.au suburb profile URL to `profile_urls`.
2. Set `maxResults` to control how many profiles are saved in the run.
3. Export each profile record and inspect the nested trend series.

```json
{
  "profile_urls": [
    {
      "url": "https://www.realestate.com.au/vic/richmond-3121/"
    }
  ],
  "maxResults": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `profile_urls` | Array\<object\> | Yes | Full realestate.com.au suburb or market profile URLs. |
| `profile_urls[].url` | string | Yes per entry | Full realestate.com.au suburb or market profile URL. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "success": true,
  "data": {
    "url": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
    "profile": {
      "resi-property_market-explorer": {
        "suburb_data": {
          "marketProfileBySlug": {
            "insights": {
              "medianPrice": {
                "buy": {
                  "house": {
                    "allBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "fourBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "unit": {
                    "allBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "oneBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "rent": {
                  "house": {
                    "allBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "fourBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "unit": {
                    "allBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "oneBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "__typename": "PropertyListing"
              },
              "transactionVolume": {
                "buy": {
                  "house": {
                    "allBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "fourBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "unit": {
                    "allBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "oneBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "rent": {
                  "house": {
                    "allBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "fourBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "unit": {
                    "allBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "oneBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "__typename": "PropertyListing"
              },
              "daysOnSite": {
                "buy": {
                  "house": {
                    "allBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "fourBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "unit": {
                    "allBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "oneBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "rent": {
                  "house": {
                    "allBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "fourBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "unit": {
                    "allBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "oneBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "__typename": "PropertyListing"
              },
              "rentalYield": {
                "house": {
                  "allBed": {
                    "yearly": {
                      "display": "Richmond, VIC 3121",
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "twoBed": {
                    "yearly": {
                      "display": "Richmond, VIC 3121",
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "threeBed": {
                    "yearly": {
                      "display": "Richmond, VIC 3121",
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "fourBed": {
                    "yearly": {
                      "display": "Richmond, VIC 3121",
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "unit": {
                  "allBed": {
                    "yearly": {
                      "display": "Richmond, VIC 3121",
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "oneBed": {
                    "yearly": {
                      "display": "Richmond, VIC 3121",
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "twoBed": {
                    "yearly": {
                      "display": "Richmond, VIC 3121",
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "threeBed": {
                    "yearly": {
                      "display": "Richmond, VIC 3121",
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "__typename": "PropertyListing"
              },
              "supplyDemand": {
                "buy": {
                  "house": {
                    "allBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "fourBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "unit": {
                    "allBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "oneBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "rent": {
                  "house": {
                    "allBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "fourBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "unit": {
                    "allBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "oneBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "__typename": "PropertyListing"
              },
              "__typename": "PropertyListing"
            },
            "market": {
              "url": {
                "href": "https://www.realestate.com.au/agent/jordan-ellis",
                "path": "https://www.realestate.com.au/agent/jordan-ellis",
                "__typename": "PropertyListing"
              },
              "__typename": "PropertyListing",
              "location": {
                "suburb": "Richmond",
                "state": "VIC",
                "postcode": "3121",
                "__typename": "PropertyListing"
              }
            },
            "breadcrumbs": [
              {
                "display": "Richmond, VIC 3121",
                "url": {
                  "path": "https://www.realestate.com.au/agent/jordan-ellis",
                  "href": "https://www.realestate.com.au/agent/jordan-ellis",
                  "__typename": "PropertyListing"
                },
                "__typename": "PropertyListing"
              }
            ],
            "__typename": "PropertyListing",
            "parentMarket": {
              "name": "Harbourline Property Group",
              "__typename": "PropertyListing"
            },
            "surroundingMarkets": [
              {
                "location": {
                  "state": "VIC",
                  "suburb": "Richmond",
                  "postcode": "3121",
                  "__typename": "PropertyListing"
                },
                "url": {
                  "path": "https://www.realestate.com.au/agent/jordan-ellis",
                  "__typename": "PropertyListing"
                },
                "__typename": "PropertyListing"
              }
            ],
            "leadGen": {
              "actionUrl": "https://www.realestate.com.au/agent/jordan-ellis",
              "data": {
                "listingCompany": {
                  "id": "152000001",
                  "name": "Harbourline Property Group",
                  "branding": {
                    "primaryColour": "Harbourline Property Group",
                    "textColour": "Harbourline Property Group",
                    "__typename": "PropertyListing"
                  },
                  "media": {
                    "logo": {
                      "url": "https://www.realestate.com.au/agent/jordan-ellis",
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "__typename": "PropertyListing"
              },
              "__typename": "PropertyListing"
            }
          }
        }
      }
    },
    "raw": {
      "resi-property_market-explorer": {
        "suburb_data": {
          "marketProfileBySlug": {
            "insights": {
              "medianPrice": {
                "buy": {
                  "house": {
                    "allBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "fourBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "unit": {
                    "allBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "oneBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "rent": {
                  "house": {
                    "allBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "fourBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "unit": {
                    "allBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "oneBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 12.8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "startDate": "2026-09-18",
                        "endDate": "2026-09-18",
                        "display": "Richmond, VIC 3121",
                        "volume": 8,
                        "changePercentage": {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "__typename": "PropertyListing"
                        },
                        "__typename": "PropertyListing"
                      },
                      "trends": [
                        {
                          "value": 8,
                          "display": "Richmond, VIC 3121",
                          "volume": 8,
                          "endDate": "2026-09-18",
                          "startDate": "2026-09-18",
                          "__typename": "PropertyListing"
                        }
                      ],
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "__typename": "PropertyListing"
              },
              "transactionVolume": {
                "buy": {
                  "house": {
                    "allBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "fourBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "unit": {
                    "allBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "oneBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "rent": {
                  "house": {
                    "allBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "fourBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "unit": {
                    "allBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "oneBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "__typename": "PropertyListing"
              },
              "daysOnSite": {
                "buy": {
                  "house": {
                    "allBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "fourBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "unit": {
                    "allBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "oneBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "rent": {
                  "house": {
                    "allBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "fourBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "unit": {
                    "allBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "oneBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "yearly": {
                        "value": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "__typename": "PropertyListing"
              },
              "rentalYield": {
                "house": {
                  "allBed": {
                    "yearly": {
                      "display": "Richmond, VIC 3121",
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "twoBed": {
                    "yearly": {
                      "display": "Richmond, VIC 3121",
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "threeBed": {
                    "yearly": {
                      "display": "Richmond, VIC 3121",
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "fourBed": {
                    "yearly": {
                      "display": "Richmond, VIC 3121",
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "unit": {
                  "allBed": {
                    "yearly": {
                      "display": "Richmond, VIC 3121",
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "oneBed": {
                    "yearly": {
                      "display": "Richmond, VIC 3121",
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "twoBed": {
                    "yearly": {
                      "display": "Richmond, VIC 3121",
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "threeBed": {
                    "yearly": {
                      "display": "Richmond, VIC 3121",
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "__typename": "PropertyListing"
              },
              "supplyDemand": {
                "buy": {
                  "house": {
                    "allBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "fourBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "unit": {
                    "allBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "oneBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "rent": {
                  "house": {
                    "allBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "fourBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "unit": {
                    "allBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "oneBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "twoBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "threeBed": {
                      "monthly": {
                        "supply": 8,
                        "demand": 8,
                        "__typename": "PropertyListing"
                      },
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "__typename": "PropertyListing"
              },
              "__typename": "PropertyListing"
            },
            "market": {
              "url": {
                "href": "https://www.realestate.com.au/agent/jordan-ellis",
                "path": "https://www.realestate.com.au/agent/jordan-ellis",
                "__typename": "PropertyListing"
              },
              "__typename": "PropertyListing",
              "location": {
                "suburb": "Richmond",
                "state": "VIC",
                "postcode": "3121",
                "__typename": "PropertyListing"
              }
            },
            "breadcrumbs": [
              {
                "display": "Richmond, VIC 3121",
                "url": {
                  "path": "https://www.realestate.com.au/agent/jordan-ellis",
                  "href": "https://www.realestate.com.au/agent/jordan-ellis",
                  "__typename": "PropertyListing"
                },
                "__typename": "PropertyListing"
              }
            ],
            "__typename": "PropertyListing",
            "parentMarket": {
              "name": "Harbourline Property Group",
              "__typename": "PropertyListing"
            },
            "surroundingMarkets": [
              {
                "location": {
                  "state": "VIC",
                  "suburb": "Richmond",
                  "postcode": "3121",
                  "__typename": "PropertyListing"
                },
                "url": {
                  "path": "https://www.realestate.com.au/agent/jordan-ellis",
                  "__typename": "PropertyListing"
                },
                "__typename": "PropertyListing"
              }
            ],
            "leadGen": {
              "actionUrl": "https://www.realestate.com.au/agent/jordan-ellis",
              "data": {
                "listingCompany": {
                  "id": "152000001",
                  "name": "Harbourline Property Group",
                  "branding": {
                    "primaryColour": "Harbourline Property Group",
                    "textColour": "Harbourline Property Group",
                    "__typename": "PropertyListing"
                  },
                  "media": {
                    "logo": {
                      "url": "https://www.realestate.com.au/agent/jordan-ellis",
                      "__typename": "PropertyListing"
                    },
                    "__typename": "PropertyListing"
                  },
                  "__typename": "PropertyListing"
                },
                "__typename": "PropertyListing"
              },
              "__typename": "PropertyListing"
            }
          }
        }
      }
    }
  },
  "profile_url": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
  "input_url": "https://www.realestate.com.au/vic/richmond-3121/",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.50 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Set **maxResults** to cap the number of dataset items saved in one run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~realestate-com-au-suburb-profile-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can I use a property listing URL?

No. Use a suburb or market profile URL. Property, agent, and agency URLs are handled by separate Actors.

## Related Scrappa Actors

- [Realestate.com.au Property Search Scraper](https://apify.com/thescrappa/realestate-com-au-search-scraper)
- [Realestate.com.au Property Details Scraper](https://apify.com/thescrappa/realestate-com-au-property-scraper)
- [Redfin Property Search Scraper](https://apify.com/thescrappa/redfin-property-search-scraper)

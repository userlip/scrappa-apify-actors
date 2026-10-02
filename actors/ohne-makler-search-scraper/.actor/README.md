# Ohne-Makler Property Search Scraper

Search Ohne-Makler by German city, postal code, county, or state. Collect listing prices, rooms, living area, location, coordinates, and links across result pages.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `id` | Integer | Numeric Ohne-Makler identifier for the property listing. |
| `object_number` | String | Public object number displayed on the Ohne-Makler listing. |
| `title` | String | Property title shown in Ohne-Makler search results. |
| `url` | String | Public Ohne-Makler URL for the property listing. |
| `price` | Integer | Advertised asking price in euros, when the seller publishes it. |
| `price_on_request` | Boolean | True when the seller asks viewers to request the price. |
| `postal_code` | String | German postal code for the listed property location. |
| `city` | String | City assigned to the listing. |
| `district` | String or null | City district for the property, when reported. |
| `street` | String or null | Street address for the listing, when published. |
| `rooms` | Number | Number of rooms advertised for the property. |
| `living_area_m2` | Integer | Advertised living area in square metres. |
| `plot_area_m2` | String or null | Land area in square metres for houses or land listings, when available. |
| `preview_image` | String | Preview image URL for the property listing. |
| `seller_type` | String or null | Seller category reported by Ohne-Makler, when available. |
| `latitude` | Number | Property latitude in decimal degrees. |
| `longitude` | Number | Property longitude in decimal degrees. |
| `input_q` | String | Q submitted to retrieve this property listing. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Home seekers can compare listings by location, room count, and living area.
- Property investors can monitor German supply by city and transaction type.
- Housing researchers can export asking prices and coordinates for market analysis.

## How to use

1. Add a German place name or postal code to `locations` for each search.
2. Choose a property type and buy or rent transaction, then set optional price and area filters.
3. Set `maxPages` and `maxResults`, then export the listing rows.

```json
{
  "locations": [
    {
      "q": "berlin"
    }
  ],
  "type": "wohnung",
  "transaction": "mieten",
  "page": 1,
  "maxResults": 5,
  "maxPages": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `locations` | Array\<object\> | Yes | City, postal code, county, or state names to resolve on Ohne-Makler. |
| `locations[].q` | string | Yes per entry | Free-text location: postal code \(80331\), city \(München\), Kreis, or Bundesland. Results are limited to the resolved place \(returned as location\). State/city segments are not accepted alongside q. |
| `locations[].type` | string | No | Property type slug: wohnung, haus, grundstueck, zimmer, buero, einzelhandel, gastronomie, lagerhalle, landwirtschaftliches-objekt, wohnen-auf-zeit, gewerbliche-freizeitimmobilie, zinshaus-rendite, sonstiges, immobilie. Required unless q is used. |
| `locations[].transaction` | string | No | kaufen \(buy\) or mieten \(rent\). Required unless q is used. |
| `locations[].state` | string | No | Bundesland URL slug \(transliterated umlauts, e.g. baden-wurttemberg\). Omit for a Germany-wide class. |
| `locations[].city` | string | No | City or Kreis URL slug; requires state. |
| `locations[].page` | integer | No | 1-based result page. Beyond the last page returns a non-billable 404. |
| `locations[].sort` | string | No | price\_asc, price\_desc, date\_asc, or date\_desc. |
| `locations[].price_min` | integer | No | Minimum price in EUR. |
| `locations[].price_max` | integer | No | Maximum price in EUR. |
| `locations[].area_min` | integer | No | Minimum living area in m². |
| `locations[].area_max` | integer | No | Maximum living area in m². |
| `locations[].radius` | integer | No | Umkreis in km: 0, 10, 25, 50, or 100. Applies when q resolves to a city, or with state + city. Postal codes, Kreise, and states have no Umkreis \(non-billable 422 radius\_requires\_city\). Allowed values: 0, 10, 25, 50, 100. |
| `type` | string | No | Property type slug: wohnung, haus, grundstueck, zimmer, buero, einzelhandel, gastronomie, lagerhalle, landwirtschaftliches-objekt, wohnen-auf-zeit, gewerbliche-freizeitimmobilie, zinshaus-rendite, sonstiges, immobilie. Required unless q is used. |
| `transaction` | string | No | kaufen \(buy\) or mieten \(rent\). Required unless q is used. |
| `state` | string | No | Bundesland URL slug \(transliterated umlauts, e.g. baden-wurttemberg\). Omit for a Germany-wide class. |
| `city` | string | No | City or Kreis URL slug; requires state. |
| `page` | integer | No | 1-based result page. Beyond the last page returns a non-billable 404. |
| `sort` | string | No | price\_asc, price\_desc, date\_asc, or date\_desc. |
| `price_min` | integer | No | Minimum price in EUR. |
| `price_max` | integer | No | Maximum price in EUR. |
| `area_min` | integer | No | Minimum living area in m². |
| `area_max` | integer | No | Maximum living area in m². |
| `radius` | integer | No | Umkreis in km: 0, 10, 25, 50, or 100. Applies when q resolves to a city, or with state + city. Postal codes, Kreise, and states have no Umkreis \(non-billable 422 radius\_requires\_city\). Allowed values: 0, 10, 25, 50, 100. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "id": 8000205,
  "object_number": "OM-583214",
  "title": "Bright two-bedroom flat in Berlin",
  "url": "https://www.ohne-makler.net/immobilie/offer/583214/",
  "price": 549,
  "price_on_request": true,
  "postal_code": "10115",
  "city": "Berlin",
  "district": "Verified source detail",
  "street": null,
  "rooms": 2,
  "living_area_m2": 78,
  "plot_area_m2": null,
  "preview_image": "https://cdn.northstar.invalid/images/aurora-front.webp",
  "seller_type": null,
  "latitude": 52.52,
  "longitude": 13.405,
  "input_q": "berlin",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~ohne-makler-search-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can I search by postal code?

Yes. The location query accepts a city, postal code, county, or German state. A radius filter applies only when the location resolves to a city.

## Related Scrappa Actors

- [Ohne-Makler Listing Details Scraper](https://apify.com/thescrappa/ohne-makler-listing-scraper)
- [Ohne-Makler Property Prices Scraper](https://apify.com/thescrappa/ohne-makler-property-prices-scraper)
- [Idealista Property Search Scraper](https://apify.com/thescrappa/idealista-search-scraper)

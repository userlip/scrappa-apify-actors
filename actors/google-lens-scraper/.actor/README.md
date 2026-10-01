# Google Lens Visual Search Scraper

Submit public image links to find similar pages and products. Rows include match titles, source labels, thumbnails, links, prices when present, and rank.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `source` | String | Website or merchant name associated with the visual match. |
| `title` | String | Product or page title associated with this Google Lens match. |
| `link` | String | Destination page URL for the visually similar result. |
| `thumbnail` | String | Preview image URL returned for the visual match. |
| `price` | String | Displayed product price and currency when Google Lens finds one. |
| `position` | Integer | Position of the visual match in the returned list, starting at 1. |
| `input_url` | String | Public image URL submitted to Google Lens for visual search. |
| `scraped_at` | String | UTC date and time when this visual match was collected. |

## Use cases

- E-commerce teams can find comparable products from catalog photos.
- Visual search specialists can map source pages and merchants for an image.
- Product researchers can compare visual matches, prices, and thumbnails.

## How to use

1. Add one public image URL to `image_urls` for each search.
2. Set `q` or `type` to refine the search when supported.
3. Review matched source, image, link, and price fields.

```json
{
  "image_urls": [
    {
      "url": "https://upload.wikimedia.org/wikipedia/commons/4/47/PNG_transparency_demonstration_1.png"
    }
  ],
  "maxResults": 20
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `image_urls` | Array\<object\> | Yes | Public image URLs to use for visual search. |
| `image_urls[].url` | string | Yes per entry | Public HTTPS URL for the image to search. |
| `image_urls[].q` | string | No | Optional text used to refine visual or product matches. |
| `image_urls[].type` | string | No | One of `all`, `products`, `exact_matches`, or `visual_matches`. |
| `q` | string | No | Optional text used to refine visual or product matches. |
| `type` | string | No | One of `all`, `products`, `exact_matches`, or `visual_matches`. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "source": "Cedar & Clay",
  "title": "Cobalt Blue Stoneware Mug",
  "link": "https://cedarandclay.co/products/cobalt-blue-stoneware-mug",
  "thumbnail": "https://cdn.cedarandclay.co/products/cobalt-mug/front-1200.jpg",
  "price": "$28.00",
  "position": 1,
  "input_url": "https://upload.wikimedia.org/wikipedia/commons/4/47/PNG_transparency_demonstration_1.png",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Set **maxResults** to cap the number of dataset items saved in one run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~google-lens-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Do all visual matches include a price?

No. A price appears only when the matched result contains price information.

## Related Scrappa Actors

- [Google Shopping Scraper](https://apify.com/thescrappa/google-shopping-scraper)
- [Apple App Store Search Scraper](https://apify.com/thescrappa/apple-app-store-search-scraper)
- [Brave Search Scraper](https://apify.com/thescrappa/brave-search-scraper)

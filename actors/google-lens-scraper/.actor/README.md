# Google Lens Visual Search Scraper

Find visually similar products and pages with Google Lens image search.

## Data you get

- **title**: Result title or listing name.
- **link**: Result destination URL.
- **source**: Publisher, seller, or source name.
- **thumbnail**: Thumbnail returned for this Google Lens Visual Search Scraper result.
- **position**: Position of the result on the source page.

## Use cases

- Visual product discovery
- Image source research
- Reverse image lookup

## How to use

Add one or more entries to **image_urls**. Each entry maps its **url** value to the Scrappa **url** input. Shared endpoint options can be set at the top level.

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

## Output example

This synthetic example shows the response fields and the input value attached to each result.

```json
{
  "source": "Example Store",
  "title": "Example blue ceramic mug",
  "link": "https://example.com/mug",
  "thumbnail": "https://example.com/mug.jpg",
  "price": "$18.00",
  "position": 1,
  "input_url": "https://upload.wikimedia.org/wikipedia/commons/4/47/PNG_transparency_demonstration_1.png",
  "scraped_at": "2026-01-01T00:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. The Actor writes one dataset item for each result.

The Actor saves up to **maxResults** dataset items across the run.

## FAQ

### Is scraping this data legal?

Scraping rules depend on the source, the data, and how you use it. Review the applicable laws, source terms, and privacy requirements for your use case. You are responsible for your collection and use of the data.

### Are there request limits?

You can submit up to 100 batch entries per run. Set **maxResults** to cap saved rows. Scrappa API limits and source availability also apply.

### Can I use the output with integrations or the API?

Yes. Read results from the Apify dataset, use the Apify API or client libraries, or connect the dataset to your existing data workflow. Each row includes **input_url** and **scraped_at** for traceability.

## Related Actors

- [Google Images Scraper](https://apify.com/thescrappa/google-images-scraper)
- [Google Shopping Scraper](https://apify.com/thescrappa/google-shopping-scraper)

## Search terms

`Google Lens Visual Search Scraper`, `title`, `link`, `source`, `/google/lens API`

# Google Images Scraper

Search Google Images and export dataset-ready image metadata for product research, visual SEO, brand monitoring, content sourcing, and machine learning dataset collection. The actor wraps Scrappa's `/api/images` endpoint.

## What you get

- Image result title, source website, source page URL, thumbnail URL, and original image URL
- Original image width and height when Google exposes dimensions
- Filters for size, type, color, aspect ratio, safe search, language, country, and time-based `tbs`
- Full Scrappa response saved to key-value store record `OUTPUT`

## Input

```json
{
  "q": "coffee product photography",
  "page": 1,
  "hl": "en",
  "gl": "us",
  "imgsz": "large",
  "imgtype": "photo",
  "imgcolor": "white",
  "imgar": "wide",
  "safe": "active"
}
```

## Output

Each dataset item is one Google Images result:

```json
{
  "position": 1,
  "thumbnail_url": "https://encrypted-tbn0.gstatic.com/images?q=tbn:...",
  "image_url": "https://example.com/images/coffee.jpg",
  "title": "Coffee product photo",
  "source": "Example Store",
  "source_url": "https://example.com/coffee",
  "width": 1920,
  "height": 1280,
  "is_product": false,
  "request_q": "coffee product photography",
  "request_gl": "us",
  "request_hl": "en",
  "request_imgsz": "large"
}
```

## Notes

For higher-volume Google Images collection or direct API access, use Scrappa's Google Images API at `https://scrappa.co/api/images`.

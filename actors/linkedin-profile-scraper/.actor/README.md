# LinkedIn Profile Scraper

Extract comprehensive public LinkedIn profile data without login. Get professional experience, education, skills, articles, recommendations, and more.

## Features

- **Profile Information** - Name, image, location, headline, followers, connections, about section
- **Experience** - Complete work history with company details, roles, and dates
- **Education** - Schools, degrees, fields of study, and graduation dates
- **Skills** - Professional skills and endorsements
- **Articles** - Published articles and content
- **Activity** - Recent posts and engagement
- **Publications** - Published works and research
- **Projects** - Professional projects and contributions
- **Recommendations** - Endorsements from colleagues and clients
- **Similar Profiles** - LinkedIn's suggestions for similar professionals
- **Caching** - Optional caching to reduce costs and improve speed

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes | LinkedIn profile URL (e.g., https://www.linkedin.com/in/williamhgates) |
| `use_cache` | boolean | No | Use cached data if available (default: true) |
| `maximum_cache_age` | integer | No | Maximum cache age in seconds (default: 2592000 = 30 days) |

## Output

### Dataset (Profile Data)

Each profile is saved to the dataset with main fields:

```json
{
  "success": true,
  "name": "Bill Gates",
  "location": "Seattle, Washington, United States",
  "followers": 35000000,
  "connections": 500,
  "about": "Co-chair of the Bill & Melinda Gates Foundation...",
  "image": "https://media.licdn.com/...",
  "job_titles": ["Co-chair, Bill & Melinda Gates Foundation"],
  "experience": [...],
  "education": [...],
  "skills": [...],
  "articles": [...],
  "activity": [...],
  "publications": [...],
  "projects": [...],
  "recommendations": [...],
  "similar_profiles": [...]
}
```

### Experience Array

```json
{
  "company": "Microsoft",
  "title": "Co-founder",
  "location": "Redmond, WA",
  "start_date": "1975",
  "end_date": "2008",
  "description": "Co-founded and led Microsoft..."
}
```

### Education Array

```json
{
  "school": "Harvard University",
  "degree": "Bachelor's degree",
  "field": "Applied Mathematics and Computer Science",
  "start_date": "1973",
  "end_date": "1975"
}
```

### Key-Value Store (Full Response)

The complete response is saved to the `OUTPUT` key with all profile data.

## Example Input

```json
{
  "url": "https://www.linkedin.com/in/williamhgates",
  "use_cache": true,
  "maximum_cache_age": 2592000
}
```

## Example Output

```json
{
  "success": true,
  "name": "Bill Gates",
  "image": "https://media.licdn.com/dms/image/...",
  "location": "Seattle, Washington, United States",
  "followers": 35000000,
  "connections": 500,
  "about": "Co-chair of the Bill & Melinda Gates Foundation. Founder of Breakthrough Energy. Co-founder of Microsoft. Voracious reader. Avid traveler. Active blogger.",
  "job_titles": ["Co-chair, Bill & Melinda Gates Foundation"],
  "experience": [
    {
      "company": "Bill & Melinda Gates Foundation",
      "title": "Co-chair",
      "location": "Seattle, Washington",
      "start_date": "2000",
      "end_date": "Present",
      "description": "Guided by the belief that every life has equal value..."
    }
  ],
  "education": [
    {
      "school": "Harvard University",
      "degree": "Bachelor's degree",
      "field": "Applied Mathematics and Computer Science",
      "start_date": "1973",
      "end_date": "1975"
    }
  ],
  "skills": ["Public Speaking", "Strategic Planning", "Software Development"],
  "articles": [...],
  "activity": [...],
  "publications": [...],
  "projects": [...],
  "recommendations": [...],
  "similar_profiles": [...]
}
```

## Pricing

$0.30 per 1,000 results. No LinkedIn login or additional API keys required.

## Use Cases

- **Recruitment** - Find and evaluate candidates for job positions
- **Lead Generation** - Build prospect lists with professional information
- **Market Research** - Analyze professional backgrounds in specific industries
- **Competitive Intelligence** - Research company employees and executives
- **Network Analysis** - Map professional connections and relationships
- **Content Research** - Find subject matter experts and thought leaders

## Support

For issues or questions, contact us through Apify.

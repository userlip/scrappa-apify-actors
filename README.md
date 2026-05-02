# Scrappa Apify Actors

Apify Actors for [Scrappa](https://scrappa.co) APIs - Google Search, Maps, YouTube, LinkedIn, and more.

## Structure

```
scrappa-apify-actors/
├── actors/           # Individual Apify actors
│   ├── google-search/
│   ├── google-maps-business-details-scraper/
│   └── ...
├── shared/           # Shared Scrappa client library
└── package.json      # Workspace root
```

## Setup

```bash
# Install pnpm if you don't have it
npm install -g pnpm

# Install dependencies
pnpm install

# Build all packages
pnpm build
```

## Development

### Running an actor locally

```bash
cd actors/google-search
pnpm start:dev
```

### Creating a new actor

1. Copy an existing actor as a template:
```bash
cp -r actors/google-search actors/my-new-actor
```

2. Update `package.json` with the new name
3. Update `.actor/actor.json` with metadata
4. Update `.actor/input_schema.json` with input fields
5. Implement the actor logic in `src/main.ts`

## Deploying to Apify

### First time setup

```bash
# Login to Apify
apify login

# Push an actor
cd actors/google-search
apify push
```

### Updating an actor

```bash
cd actors/google-search
pnpm build
apify push
```

## Available Actors

This repository contains source directories for the following live Scrappa Apify Actors. Backlog ideas are intentionally excluded from this inventory until they have a deployed Actor ID.

| Local directory | Apify actor | Actor ID | Description | Status |
|-----------------|-------------|----------|-------------|--------|
| `actors/google-maps-advanced-search-scraper` | `google-maps-advanced-search-scraper` | `DT8bUdm2Vn4HjlyDo` | Google Maps coordinate-based advanced search | Live on Apify |
| `actors/google-maps-autocomplete-scraper` | `google-maps-autocomplete-scraper` | `hhS8GkceJHFiexWe6` | Google Maps autocomplete suggestions | Live on Apify |
| `actors/google-maps-business-details-scraper` | `google-maps-business-details-scraper` | `JCqaAyY3Vy7K5UoRd` | Google Maps business details | Live on Apify |
| `actors/google-maps-photos-scraper` | `google-maps-photos-scraper` | `gLbfii9Nq4H7auMnN` | Google Maps business photos | Live on Apify |
| `actors/google-maps-reviews-scraper` | `google-maps-reviews-scraper` | `QvxzSeJiQrMggt1Vn` | Google Maps business reviews | Live on Apify |
| `actors/google-maps-search-scraper` | `google-maps-search-scraper` | `3fXhf8bJruXVWgDKy` | Google Maps business search | Live on Apify |
| `actors/google-search` | `google-search-scraper` | `2pU7EbKhShUz8BAnN` | Google Search results | Live on Apify |
| `actors/instagram-post-info-cheapest-0-20-1000-results` | `instagram-post-info-cheapest-0-20-1000-results` | `nfdzs1z0cRIU1Bfhw` | Instagram post details | Live on Apify |
| `actors/instagram-user-info-cheapest-0-20-1000-results` | `instagram-user-info-cheapest-0-20-1000-results` | `VZrsJ6bO3h92y0duj` | Instagram user profile details | Live on Apify |
| `actors/instagram-user-posts-cheapest-0-20-1000-results` | `instagram-user-posts-cheapest-0-20-1000-results` | `mp03zGSA2pR31azfU` | Instagram user posts and reels | Live on Apify |
| `actors/linkedin-company-scraper` | `linkedin-company-scraper` | `EMGCTVXuOBRERiDMf` | LinkedIn company pages | Live on Apify |
| `actors/linkedin-post-scraper` | `linkedin-post-scraper` | `hVDOXgRoKJbnATxzs` | LinkedIn posts and articles | Live on Apify |
| `actors/linkedin-profile-scraper` | `linkedin-profile-scraper` | `87AaxKjjQrK0F0g60` | LinkedIn public profiles | Live on Apify |
| `actors/scrappa-google-search` | `scrappa-google-search` | `8ejIZ0nfRPShvWBSP` | Scrappa Google Search | Live on Apify |
| `actors/youtube-api-search-data` | `youtube-api-search-data` | `ziD2fUoLsdzKlc6zR` | YouTube search data | Live on Apify |

## API Key

All actors require a Scrappa API key. Get yours at [https://scrappa.co](https://scrappa.co).

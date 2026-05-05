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

This inventory is aligned to the live `TheScrappa` Apify org as of 2026-05-05. It lists all 40 live actors, including actors that are currently missing a matching local source directory in this repo.

Current coverage in this repository:
- 40 live actors in Apify
- 24 local actor directories in this repo
- 17 live actors missing a local source directory here
- 1 live actor pending Store publication and pricing

| Local directory | Apify actor | Actor ID | Title | Source coverage |
|---|---|---|---|---|
| `actors/google-jobs-scraper` | `google-jobs-scraper` | `4DwyH8vZcinXWrHBA` | Google Jobs Scraper | Local source present + live on Apify |
| `actors/google-maps-advanced-search-scraper` | `google-maps-advanced-search-scraper` | `DT8bUdm2Vn4HjlyDo` | Google Maps Advanced Search Scraper | Local source present + live on Apify |
| `actors/google-maps-autocomplete-scraper` | `google-maps-autocomplete-scraper` | `hhS8GkceJHFiexWe6` | Google Maps Autocomplete Scraper | Local source present + live on Apify |
| `actors/google-maps-business-details-scraper` | `google-maps-business-details-scraper` | `JCqaAyY3Vy7K5UoRd` | Google Maps Business Details Scraper | Local source present + live on Apify |
| `actors/google-maps-photos-scraper` | `google-maps-photos-scraper` | `gLbfii9Nq4H7auMnN` | Google Maps Photos Scraper | Local source present + live on Apify |
| `actors/google-maps-reviews-scraper` | `google-maps-reviews-scraper` | `QvxzSeJiQrMggt1Vn` | Google Maps Reviews Scraper | Local source present + live on Apify |
| `actors/google-maps-search-scraper` | `google-maps-search-scraper` | `3fXhf8bJruXVWgDKy` | Google Maps Search Scraper | Local source present + live on Apify |
| `actors/google-news-scraper` | `google-news-scraper` | `HYG9AqNEDSHMHgH4O` | Google News Scraper | Local source present + live on Apify |
| `actors/google-search` | `google-search-scraper` | `2pU7EbKhShUz8BAnN` | Google Search Scraper | Local source present + live on Apify |
| `actors/instagram-post-info-cheapest-0-20-1000-results` | `instagram-post-info-cheapest-0-20-1000-results` | `nfdzs1z0cRIU1Bfhw` | Instagram Post Info \| Cheapest $0.20/1k results | Local source present + live on Apify |
| `actors/instagram-user-info-cheapest-0-20-1000-results` | `instagram-user-info-cheapest-0-20-1000-results` | `VZrsJ6bO3h92y0duj` | Instagram User Info \| Cheapest $0.20/1k results | Local source present + live on Apify |
| `actors/instagram-user-posts-cheapest-0-20-1000-results` | `instagram-user-posts-cheapest-0-20-1000-results` | `mp03zGSA2pR31azfU` | Instagram User Posts Cheapest 0 20 1000 Results | Local source present + live on Apify |
| `actors/linkedin-company-scraper` | `linkedin-company-scraper` | `EMGCTVXuOBRERiDMf` | LinkedIn Company Scraper - $0.30/1k results | Local source present + live on Apify |
| `actors/linkedin-post-scraper` | `linkedin-post-scraper` | `hVDOXgRoKJbnATxzs` | LinkedIn Post Scraper - $0.30/1k results | Local source present + live on Apify |
| `actors/linkedin-profile-scraper` | `linkedin-profile-scraper` | `87AaxKjjQrK0F0g60` | LinkedIn Profile Scraper - $0.30/1k results | Local source present + live on Apify |
| `actors/scrappa-google-search` | `scrappa-google-search` | `8ejIZ0nfRPShvWBSP` | Scrappa Google Search | Local source present + live on Apify |
| `actors/tiktok-comments-scraper` | `tiktok-comments-scraper` | `oaJANlheGg9o3EZjU` | Tiktok Comments Scraper | Local source present + live on Apify |
| `actors/tiktok-profile-scraper` | `tiktok-profile-scraper` | `ElkkSkWZ7xAaOqsr4` | Tiktok Profile Scraper | Local source present + live on Apify |
| `actors/youtube-api-batch-videos` | `youtube-api-batch-videos` | `6ZUj6u4SWuJxOQnn9` | Youtube API Batch Videos | Local source present + live on Apify |
| `Missing locally` | `youtube-api-channel-podcasts` | `Y3mKYlGNhsrBE7aZO` | Youtube Api Channel Podcasts | Live on Apify, source directory not in this repo |
| `Missing locally` | `youtube-api-channel-videos` | `w464EbPGGZqcmrC8j` | Youtube Api Channel Videos | Live on Apify, source directory not in this repo |
| `Missing locally` | `youtube-api-get-channel-about-details` | `vKqlzEXa47Ubpuix5` | Youtube Api Get Channel "About" Details | Live on Apify, source directory not in this repo |
| `Missing locally` | `youtube-api-get-channel-community` | `S9Gf6PSqzz6hxvMNA` | Youtube Api Get Channel Community | Live on Apify, source directory not in this repo |
| `Missing locally` | `youtube-api-get-channel-details` | `svtEvWGEssObsU72e` | Youtube Api Get Channel Details | Live on Apify, source directory not in this repo |
| `Missing locally` | `youtube-api-get-channel-livestreams` | `WT3XhaJ0lUYmp0eFu` | Youtube Api Get Channel Livestreams | Live on Apify, source directory not in this repo |
| `Missing locally` | `youtube-api-get-channel-playlists` | `3ERhmU2MUBjdR4AOq` | Youtube Api Get Channel Playlists | Live on Apify, source directory not in this repo |
| `Missing locally` | `youtube-api-get-channel-shorts` | `608amsD2lD6xRKbax` | Youtube Api Get Channel Shorts | Live on Apify, source directory not in this repo |
| `Missing locally` | `youtube-api-get-channel-statistics` | `yJiDippxXaK5hWQRC` | Youtube API Get Channel Statistics | Live on Apify, source directory not in this repo |
| `Missing locally` | `youtube-api-get-playlists-details` | `Xhwtx7clQKnPRez1H` | Youtube Api Get Playlists details | Live on Apify, source directory not in this repo |
| `actors/youtube-api-get-video-details` | `youtube-api-get-video-details` | `P1Jv1QuMoId4XUPlC` | Youtube API Get Video Details | Local source present + live on Apify |
| `Missing locally` | `youtube-api-hashtags` | `N5ol78TtqiMj4MtM6` | Youtube API Hashtags | Live on Apify, source directory not in this repo |
| `Missing locally` | `youtube-api-playlists` | `hueJrwkrbo20Ufrna` | Youtube API Playlists | Live on Apify, source directory not in this repo |
| `Missing locally` | `youtube-api-related-videos` | `eKzA6GhEOJACIiCUW` | Youtube API Related videos | Live on Apify, source directory not in this repo |
| `Missing locally` | `youtube-api-search-by-category` | `O1ltDU9qk4adR2x86` | Youtube API Search By Category | Live on Apify, source directory not in this repo |
| `actors/youtube-api-search-data` | `youtube-api-search-data` | `ziD2fUoLsdzKlc6zR` | YouTube Search Data Scraper | Local source present + live on Apify |
| `Missing locally` | `youtube-api-search-suggestions` | `oecJ81oeff1KozCtd` | Youtube API Search Suggestions | Live on Apify, source directory not in this repo |
| `actors/youtube-transcript-scraper` | `youtube-transcript-scraper` | `ztc698cHC09lkCDYE` | YouTube Transcript Scraper | Local source present + live on Apify; pending Store publication and pricing |
| `Missing locally` | `youtube-api-trending-videos` | `T7ddx0tgVCwMHi9ET` | Youtube API Trending Videos | Live on Apify, source directory not in this repo |
| `Missing locally` | `youtube-api-video-chapters` | `fq5Kq9OfBRWYu9go1` | Youtube API Video Chapters | Live on Apify, source directory not in this repo |
| `actors/youtube-api-video-comments` | `youtube-api-video-comments` | `ZT2Z352FLhgqgtMrg` | Youtube API Video Comments | Local source present + live on Apify |

## API Key

All actors require a Scrappa API key. Get yours at [https://scrappa.co](https://scrappa.co).

import { Actor } from 'apify';
import { errorMessage, SCRAPPA_REQUEST_TIMEOUT_MS } from './errors.js';
import { buildTrendingRequest, continuationToken, trendingVideosToDatasetItems } from './trending-url.js';

Actor.main(async () => {
    try {
        const input = (await Actor.getInput()) ?? {};
        const request = buildTrendingRequest(input);

        console.log(`Fetching from: ${request.url}`);
        const response = await fetch(request.url, {
            headers: {
                // This legacy trending endpoint is public and does not require SCRAPPA_API_KEY.
                'Accept': 'application/json',
            },
            signal: AbortSignal.timeout(SCRAPPA_REQUEST_TIMEOUT_MS),
        });
        if (!response.ok) {
            throw new Error(`Scrappa API request failed with ${response.status} ${response.statusText}`);
        }

        const data = await response.json();
        const videos = trendingVideosToDatasetItems(data);

        await Actor.pushData(videos);
        console.log(`Successfully fetched ${videos.length} trending video(s) for category: ${request.category ?? 'default'}`);

        const continuation = continuationToken(data);
        if (continuation) {
            console.log(`Continuation token available for next page: ${continuation}`);
        }
    } catch (error) {
        const message = errorMessage(error);
        console.error(`Failed to fetch YouTube trending videos: ${message}`);
        await Actor.fail(message);
    }
});

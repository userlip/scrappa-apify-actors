import { Actor } from 'apify';
import axios from 'axios';
import {
    buildChannelShortsUrl,
    buildScrappaRequest,
    getChannelIds,
    getScrappaApiKey,
} from './youtube-request.js';

function shortVideos(responseData) {
    const videos = responseData?.videos ?? [];
    return videos.filter((video) => video?.isShort === true || String(video?.type ?? '').toLowerCase() === 'short');
}

async function getChannelShorts(input, apiKey) {
    const { apiUrl, requestOptions } = buildScrappaRequest(buildChannelShortsUrl(input), apiKey);
    try {
        console.log(`Fetching from: ${apiUrl}`);
        const response = await axios.get(apiUrl, requestOptions);
        const data = shortVideos(response.data);
        
        // Save the fetched data to the default dataset.
        await Actor.pushData(data);
        console.log(`Successfully fetched ${data.length} videos for query: ${input.id}`);
        
        // Log if there's a continuation token for next page
        if (response.data.continuation) {
            console.log(`Continuation token available for next page: ${response.data.continuation}`);
        }
    } catch (error) {
        console.error(`Failed to fetch Shorts videos for channel id: ${input.id}`, error.message);
        throw error;
    }
}

Actor.main(async () => {
    await Actor.init();

    const apiKey = getScrappaApiKey();
    const input = (await Actor.getInput()) || {};
    const ids = getChannelIds(input);

    if (ids.length === 0) {
        throw new Error('At least one YouTube channel ID must be provided in "ids" or "id".');
    }

    for (const id of ids) {
        await getChannelShorts({ ...input, id }, apiKey);
    }

    await Actor.exit();
});

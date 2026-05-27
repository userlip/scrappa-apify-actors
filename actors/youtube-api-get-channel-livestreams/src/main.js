import { Actor } from 'apify';
import axios from 'axios';
import {
    assertContinuationMatchesBatch,
    buildChannelLivestreamsUrl,
    buildScrappaRequest,
    collectFilteredChannelVideos,
    getChannelIds,
    getScrappaApiKey,
} from './youtube-request.js';

function isLivestreamVideo(video) {
    const type = String(video?.type ?? video?.videoType ?? '').toLowerCase();
    return video?.isLive === true || type === 'live' || type === 'livestream';
}

async function getChannelLivestreams(input, apiKey) {
    try {
        const result = await collectFilteredChannelVideos(input, async (pageInput) => {
            const { apiUrl, requestOptions } = buildScrappaRequest(buildChannelLivestreamsUrl(pageInput), apiKey);
            console.log(`Fetching from: ${apiUrl}`);
            const response = await axios.get(apiUrl, requestOptions);
            return response.data;
        }, isLivestreamVideo);
        const data = result.videos;
        
        // Save the fetched data to the default dataset.
        await Actor.pushData(data);
        console.log(`Successfully fetched ${data.length} videos for query: ${input.id}`);
        
        // Log if there's a continuation token for next page
        if (result.continuation) {
            console.log(`Continuation token available for next page: ${result.continuation}`);
        }
    } catch (error) {
        console.error(`Failed to fetch livestream videos for channel id: ${input.id}`, error.message);
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
    assertContinuationMatchesBatch(input, ids);

    for (const id of ids) {
        await getChannelLivestreams({ ...input, id }, apiKey);
    }

    await Actor.exit();
});

import { Actor } from 'apify';
import axios from 'axios';
import {
    assertNoUnsupportedContinuation,
    buildChannelPlaylistsUrl,
    buildScrappaRequest,
    getChannelIds,
    getScrappaApiKey,
} from './youtube-request.js';

async function getChannelPlaylists(input, apiKey) {
    const { apiUrl, requestOptions } = buildScrappaRequest(buildChannelPlaylistsUrl(input), apiKey);
    try {
        console.log(`Fetching from: ${apiUrl}`);
        const response = await axios.get(apiUrl, requestOptions);
        const data = response.data.playlists;
        
        // Save the fetched data to the default dataset.
        await Actor.pushData(data);
        console.log(`Successfully fetched ${data.length} playlists for query: ${input.id}`);
        
        // Log if there's a continuation token for next page
        if (response.data.continuation) {
            console.log(`Continuation token available for next page: ${response.data.continuation}`);
        }
    } catch (error) {
        console.error(`Failed to fetch playlists for channel id: ${input.id}`, error.message);
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
    assertNoUnsupportedContinuation(input);

    for (const id of ids) {
        await getChannelPlaylists({ ...input, id }, apiKey);
    }

    await Actor.exit();
});

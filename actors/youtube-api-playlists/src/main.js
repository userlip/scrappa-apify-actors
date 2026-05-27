import { Actor } from 'apify';
import axios from 'axios';
import {
    buildPlaylistSearchUrl,
    buildScrappaRequest,
    getContinuationToken,
    getScrappaApiKey,
} from './youtube-request.js';

/**
 * Searches for playlists based on a query string.
 * This is the main function for this Actor, as defined by the input schema.
 * @param {string} query The search query string.
 * @param {string} sort Sort order (optional)
 * @param {number} limit Number of results to return (optional)
 * @param {string} continuation Continuation token for pagination (optional)
 */
async function searchPlaylists(query, sort = 'relevance', limit = 20, continuation = '', apiKey) {
    console.log(`Running action: Search Playlists for query: "${query}"`);
    const { apiUrl, requestOptions } = buildScrappaRequest(
        buildPlaylistSearchUrl({ q: query, sort, limit, continuation }),
        apiKey,
    );

    try {
        console.log(`Fetching from: ${apiUrl}`);
        const response = await axios.get(apiUrl, requestOptions);
        const data = response.data['results'];
        
        // Save the fetched data to the default dataset.
        await Actor.pushData(data);
        console.log(`Successfully fetched ${data.length} playlists for query: ${query}`);
        
        // Log if there's a continuation token for next page
        const continuationToken = getContinuationToken(response.data);
        if (continuationToken) {
            console.log(`Continuation token available for next page: ${continuationToken}`);
        }
    } catch (error) {
        console.error(`Failed to fetch playlists for query: ${query}`, error.message);
        throw error;
    }
}

Actor.main(async () => {
    await Actor.init();

    const apiKey = getScrappaApiKey();
    const input = (await Actor.getInput()) || {};
    const { q, sort, limit, continuation } = input;

    await searchPlaylists(q, sort, limit, continuation, apiKey);

    await Actor.exit();
});

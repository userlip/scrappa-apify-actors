import { Actor } from 'apify';
import axios from 'axios';
import {
    buildHashtagSearchUrl,
    buildScrappaRequest,
    getContinuationToken,
    getScrappaApiKey,
    SCRAPPA_REQUEST_TIMEOUT_MS,
} from './youtube-request.js';

function errorMessage(error) {
    const rawMessage = error instanceof Error ? error.message : String(error);
    if (rawMessage.includes('timeout') || rawMessage.includes('aborted')) {
        return `Scrappa API request timed out after ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s`;
    }

    return rawMessage;
}

async function searchHashtag(query, sort = 'relevance', limit, duration, upload_date, continuation = '', contentType, features) {
    const apiKey = getScrappaApiKey();
    const { apiUrl, requestOptions } = buildScrappaRequest(
        buildHashtagSearchUrl({ hashtag: query, sort, limit, duration, upload_date, continuation, contentType, features }),
        apiKey,
    );

    try {
        console.log(`Fetching from: ${apiUrl}`);
        const response = await axios.get(apiUrl, requestOptions);
        const data = response.data['results'];
        
        // Save the fetched data to the default dataset.
        await Actor.pushData(data);
        console.log(`Successfully fetched ${data.length} hashtag for query: ${query}`);
        
        // Log if there's a continuation token for next page
        const continuationToken = getContinuationToken(response.data);
        if (continuationToken) {
            console.log(`Continuation token available for next page: ${continuationToken}`);
        }
    } catch (error) {
        console.error(`Failed to fetch hashtag for query: ${query}: ${errorMessage(error)}`);
        throw error;
    }
}

Actor.main(async () => {
    await Actor.init();

    const input = (await Actor.getInput()) || {};
    const { hashtag, sort, duration, upload_date, limit, continuation, contentType, features } = input;

    await searchHashtag(hashtag, sort, limit, duration, upload_date, continuation, contentType, features);

    await Actor.exit();
});

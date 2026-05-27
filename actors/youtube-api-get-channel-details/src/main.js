import { Actor } from 'apify';
import axios from 'axios';
import {
    buildChannelDetailsUrl,
    buildScrappaRequest,
    getChannelIds,
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

async function getChannelDetails(id, apiKey) {
    const { apiUrl, requestOptions } = buildScrappaRequest(buildChannelDetailsUrl(id), apiKey);
    try {
        console.log(`Fetching from: ${apiUrl}`);
        const response = await axios.get(apiUrl, requestOptions);
        return response.data;
    } catch (error) {
        console.error(`Failed to fetch YouTube channel details for id ${id}: ${errorMessage(error)}`);
        throw error;
    }
}

// Main Actor logic
Actor.main(async () => {
    await Actor.init();

    const apiKey = getScrappaApiKey();
    const input = (await Actor.getInput()) ?? {};
    const ids = getChannelIds(input);

    if (ids.length === 0) {
        throw new Error('At least one YouTube channel ID must be provided in "ids" or "id".');
    }

    let successCount = 0;
    let failureCount = 0;

    for (const id of ids) {
        try {
            const data = await getChannelDetails(id, apiKey);
            await Actor.pushData(data);
            successCount += 1;
        } catch (error) {
            failureCount += 1;
            await Actor.pushData({
                id,
                error: errorMessage(error),
                success: false,
            });
        }
    }

    if (successCount === 0) {
        throw new Error(`Failed to fetch details for all ${failureCount} channel(s).`);
    }

    console.log(`Successfully fetched details for ${successCount} channel(s); ${failureCount} failed.`);

    await Actor.exit();
});

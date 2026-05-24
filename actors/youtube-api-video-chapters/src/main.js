import { Actor } from 'apify';
import axios from 'axios';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;

function errorMessage(error) {
    const rawMessage = error instanceof Error ? error.message : String(error);
    if (rawMessage.includes('timeout') || rawMessage.includes('aborted')) {
        return `Scrappa API request timed out after ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s`;
    }

    return rawMessage;
}

function parseIds(value) {
    if (Array.isArray(value)) {
        return value.flatMap((item) => parseIds(item));
    }

    if (typeof value !== 'string') {
        return [];
    }

    return value
        .split(',')
        .map((id) => id.trim())
        .filter(Boolean);
}

function getVideoIds(input) {
    const ids = [...parseIds(input.ids), ...parseIds(input.id)];
    return [...new Set(ids)];
}

async function searchVideoChapters(id) {
    // Validate that the required query parameter is present.
    if (!id) {
        throw new Error('Video "id" not provided in input.');
    }

    // Construct the base API URL with required parameters
    const apiUrl = `https://ytapi.scrappa.co/videos/chapters?id=${encodeURIComponent(id)}`;

    try {
        console.log(`Fetching from: ${apiUrl}`);
        const response = await axios.get(apiUrl, {
            timeout: SCRAPPA_REQUEST_TIMEOUT_MS,
        });
        return response.data;
    } catch (error) {
        console.error(`Failed to fetch YouTube video chapters for id ${id}: ${errorMessage(error)}`);
        throw error;
    }
}

// Main Actor logic
Actor.main(async () => {
    // The init() call configures the Actor for its environment.
    await Actor.init();

    const input = (await Actor.getInput()) || {};
    const ids = getVideoIds(input);

    if (ids.length === 0) {
        throw new Error('At least one YouTube video ID must be provided in "ids" or "id".');
    }

    let successCount = 0;
    let failureCount = 0;

    for (const id of ids) {
        try {
            const data = await searchVideoChapters(id);
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
        throw new Error(`Failed to fetch chapters for all ${failureCount} video(s).`);
    }

    console.log(`Successfully fetched chapters for ${successCount} video(s); ${failureCount} failed.`);

    // Gracefully exit the Actor process.
    await Actor.exit();
});

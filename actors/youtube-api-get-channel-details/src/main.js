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

async function getChannelDetails(id) {
    // Validate that the required query parameter is present.
    if (!id) {
        throw new Error('Channel "id" not provided in input.');
    }

    // Construct the base API URL with required parameters
    const apiUrl = `https://ytapi.scrappa.co/channels?id=${encodeURIComponent(id)}`;

    try {
        console.log(`Fetching from: ${apiUrl}`);
        const response = await axios.get(apiUrl, {
            timeout: SCRAPPA_REQUEST_TIMEOUT_MS,
        });
        const data = response.data;

        // Save the fetched data to the default dataset.
        await Actor.pushData(data);
    } catch (error) {
        console.error(`Failed to fetch YouTube channel details for id ${id}: ${errorMessage(error)}`);
        throw error;
    }
}

// Main Actor logic
Actor.main(async () => {
    // The init() call configures the Actor for its environment.
    await Actor.init();

    const input = (await Actor.getInput()) ?? {};
    const { id } = input;

    // Directly call the function with the input, as there is only one possible task.
    await getChannelDetails(id);

    // Gracefully exit the Actor process.
    await Actor.exit();
});

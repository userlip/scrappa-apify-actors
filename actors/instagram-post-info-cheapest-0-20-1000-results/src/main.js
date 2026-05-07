// main.js
import axios from 'axios';
import { Actor } from 'apify';
import { resolveInstagramPostInput } from './input.js';
import { getResponseMessage, requestWithRetries } from './retry.js';

await Actor.init();

try {
    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
    }

    const input = await Actor.getInput();
    const { identifier, params } = resolveInstagramPostInput(input);

    const apiUrl = 'https://scrappa.co/api/instagram/post';

    const response = await requestWithRetries(async () => {
        const scrappaResponse = await axios.get(apiUrl, {
            params,
            headers: {
                'X-API-Key': apiKey,
                Accept: 'application/json',
            },
            timeout: 60000,
        });

        if (scrappaResponse.data?.success === false) {
            const error = new Error(
                `Scrappa Instagram Post API returned an error response: ${getResponseMessage(scrappaResponse.data)}`,
            );
            error.response = {
                status: scrappaResponse.data?.status_code,
                data: scrappaResponse.data,
            };
            throw error;
        }

        return scrappaResponse;
    }, {
        onRetry: (error, attempt, delayMs) => {
            const status = error?.response?.status;
            const responseMessage = getResponseMessage(error?.response?.data);
            console.warn(
                `Scrappa Instagram Post API transient failure${status ? ` (${status})` : ''}: `
                + `${responseMessage}. Retry ${attempt} in ${Math.round(delayMs / 1000)}s.`,
            );
        },
    });
    const data = response.data;

    await Actor.pushData(data);

    const store = await Actor.openKeyValueStore();
    await store.setValue('OUTPUT', data);

    console.log(`Successfully fetched data for Instagram post: ${identifier}`);
} catch (error) {
    let message = error instanceof Error ? error.message : String(error);

    if (axios.isAxiosError(error)) {
        const status = error.response?.status;
        const responseMessage = error.response?.data
            ? getResponseMessage(error.response.data)
            : error.response?.statusText;

        message = `Scrappa Instagram Post API request failed${status ? ` (${status})` : ''}: ${responseMessage ?? message}`;
    }

    console.error('Actor failed: ' + message);
    await Actor.fail(message);
}

await Actor.exit();

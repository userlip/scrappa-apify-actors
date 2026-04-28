// main.js
import axios from 'axios';
import { Actor } from 'apify';

await Actor.init();

try {
    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
    }

    const input = await Actor.getInput();
    const identifier = input?.url ?? input?.shortcode ?? input?.media_id;

    if (!identifier) {
        throw new Error('Instagram post URL or shortcode is required. Provide url, shortcode, or media_id in the input.');
    }

    const apiUrl = 'https://scrappa.co/api/instagram/post';
    const params = input?.url
        ? { url: input.url }
        : { shortcode: identifier };

    const response = await axios.get(apiUrl, {
        params,
        headers: {
            'X-API-Key': apiKey,
            Accept: 'application/json',
        },
        timeout: 60000,
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
        const responseMessage = error.response?.data?.message
            ?? error.response?.data?.error
            ?? error.response?.statusText;

        message = `Scrappa Instagram Post API request failed${status ? ` (${status})` : ''}: ${responseMessage ?? message}`;
    }

    console.error('Actor failed: ' + message);
    await Actor.fail(message);
}

await Actor.exit();

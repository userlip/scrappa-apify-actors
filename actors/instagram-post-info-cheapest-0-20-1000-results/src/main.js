// main.js
import axios from 'axios';
import { Actor } from 'apify';
import { resolveInstagramPostInput } from './input.js';
import { fetchPost } from './post.js';
import {
    getResponseStatus,
    getResponseMessage,
    isCooldownAuthScrappaError,
    isRateLimitScrappaError,
    isTransientScrappaError,
    requestWithRetries,
} from './retry.js';

await Actor.init();

try {
    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
    }

    const input = await Actor.getInput();
    const { identifier, params } = resolveInstagramPostInput(input);

    let sawRateLimitDuringRequest = false;
    let nextRetryReason = 'transient failure';

    const response = await requestWithRetries(() => fetchPost(axios.get, params, apiKey, () => {
        console.warn('Single-post lookup unavailable; checking the account feed for the exact shortcode.');
    }), {
        shouldRetry: (error) => {
            if (isTransientScrappaError(error)) {
                if (isRateLimitScrappaError(error)) {
                    sawRateLimitDuringRequest = true;
                }
                nextRetryReason = 'transient failure';
                return true;
            }

            if (sawRateLimitDuringRequest && isCooldownAuthScrappaError(error)) {
                nextRetryReason = 'cooldown auth response';
                return true;
            }

            return false;
        },
        onRetry: (error, attempt, delayMs) => {
            const status = getResponseStatus(error);
            const responseMessage = getResponseMessage(error?.response?.data);
            console.warn(
                `Scrappa Instagram Post API ${nextRetryReason}${status ? ` (${status})` : ''}: `
                + `${responseMessage}. Retry ${attempt} in ${(delayMs / 1000).toFixed(1)}s.`,
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
        const responseData = error.response?.data;
        const responseMessage = responseData && typeof responseData === 'object'
            ? getResponseMessage(responseData)
            : error.response?.statusText;

        message = `Scrappa Instagram Post API request failed${status ? ` (${status})` : ''}: ${responseMessage ?? message}`;
    }

    console.error('Actor failed: ' + message);
    await Actor.fail(message);
}

await Actor.exit();

import { Actor } from 'apify';
import { getScrappaApiKey } from './api-key.js';
import { fetchTranscript, SCRAPPA_REQUEST_TIMEOUT_MS } from './transcript-client.js';

function errorMessage(error) {
    const rawMessage = error instanceof Error ? error.message : String(error);
    if (rawMessage.includes('aborted')) {
        return `Scrappa API request timed out after ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s`;
    }

    return rawMessage;
}

Actor.main(async () => {
    try {
        const apiKey = getScrappaApiKey();
        const input = (await Actor.getInput()) ?? {};
        const { data } = await fetchTranscript(input, {
            apiKey,
            onRequest: (apiUrl) => console.log(`Fetching from: ${apiUrl}`),
        });
        const transcript = Array.isArray(data?.transcript) ? data.transcript : [];

        await Actor.pushData({
            ...data,
            videoId: data?.videoId ?? data?.id ?? input.id ?? null,
            transcript,
            segmentCount: transcript.length,
        });

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', data);

        console.log(`Successfully fetched transcript with ${transcript.length} segment(s) for video id: ${input.id}`);
    } catch (error) {
        const message = errorMessage(error);
        console.error(`Failed to fetch YouTube transcript: ${message}`);
        await Actor.fail(message);
    }
});

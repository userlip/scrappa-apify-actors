import { setTimeout as sleep } from 'node:timers/promises';

export const REQUEST_TIMEOUT_MS = 60000;
const MAX_ATTEMPTS = 3;

export async function fetchBatchVideos(url, request = fetch, wait = sleep) {
    for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt += 1) {
        let response;
        try {
            response = await request(url, { signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS) });
        } catch (error) {
            if (attempt === MAX_ATTEMPTS) throw error;
            console.warn(`Scrappa API request failed: ${error instanceof Error ? error.message : String(error)}; retrying (${attempt}/${MAX_ATTEMPTS})`);
            await wait(attempt * 1000);
            continue;
        }

        if (!response.ok) {
            const error = new Error(`Scrappa API request failed with ${response.status} ${response.statusText}`);
            if (attempt === MAX_ATTEMPTS || (response.status !== 429 && response.status < 500)) throw error;
            console.warn(`${error.message}; retrying (${attempt}/${MAX_ATTEMPTS})`);
            await wait(attempt * 1000);
            continue;
        }

        const data = await response.json();
        if (!Array.isArray(data?.videos)) {
            throw new Error('Scrappa API response is missing the videos array');
        }
        return data.videos;
    }
}

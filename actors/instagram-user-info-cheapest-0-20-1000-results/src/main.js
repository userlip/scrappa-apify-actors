import { Actor } from 'apify';

const SCRAPPA_API_URL = 'https://scrappa.co/api/instagram/user';
const REQUEST_TIMEOUT_MS = 90000;

function normalizeUsername(username) {
    return username.trim().replace(/^@+/, '');
}

function flattenProfile(response) {
    const user = response?.user ?? response?.data?.user ?? response?.data ?? response;

    if (!user || typeof user !== 'object') {
        return response;
    }

    return {
        ...response,
        ...user,
    };
}

function getResponseMessage(data) {
    return data?.message
        ?? data?.error
        ?? 'Unknown Scrappa API error';
}

function isAuthenticationFailure(status, data) {
    const code = typeof data?.code === 'string' ? data.code.toLowerCase() : '';
    const message = String(getResponseMessage(data)).toLowerCase();

    return status === 401
        || status === 403
        || code.includes('unauthorized')
        || code.includes('forbidden')
        || message.includes('authentication required')
        || message.includes('unauthorized')
        || message.includes('invalid api key')
        || message.includes('forbidden');
}

function parseResponseBody(body, status) {
    if (!body) {
        return { message: `HTTP ${status}` };
    }

    try {
        return JSON.parse(body);
    } catch {
        return { message: body };
    }
}

Actor.main(async () => {
    const input = await Actor.getInput();
    const username = typeof input?.username === 'string' ? normalizeUsername(input.username) : '';

    if (!username) {
        await Actor.fail('Instagram username is required.');
        return;
    }

    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        await Actor.fail('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        return;
    }

    console.log(`Fetching Instagram user info for: ${username}`);

    try {
        const url = new URL(SCRAPPA_API_URL);
        url.searchParams.set('username', username);

        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);

        let response;
        let body;
        try {
            response = await fetch(url, {
                method: 'GET',
                headers: {
                    'X-API-KEY': apiKey,
                    Accept: 'application/json',
                },
                signal: controller.signal,
            });
            body = await response.text();
        } finally {
            clearTimeout(timeoutId);
        }

        const data = parseResponseBody(body, response.status);

        if (isAuthenticationFailure(response.status, data)) {
            await Actor.fail(`Scrappa API authentication failed: ${getResponseMessage(data)}. Check the SCRAPPA_API_KEY Actor secret.`);
            return;
        }

        if (response.status >= 500) {
            await Actor.fail(`Scrappa API returned HTTP ${response.status}: ${getResponseMessage(data)}`);
            return;
        }

        if (response.status >= 400 || data?.success === false) {
            const statusPrefix = response.status >= 400
                ? `HTTP ${response.status}`
                : 'an error response';
            await Actor.fail(`Scrappa API returned ${statusPrefix}: ${getResponseMessage(data)}`);
            return;
        }

        const item = flattenProfile(data);
        await Actor.pushData(item);
        console.log(`Successfully fetched Instagram user info for: ${username}`);
    } catch (error) {
        if (error instanceof Error && error.name === 'AbortError') {
            await Actor.fail(`Scrappa API request timed out after ${REQUEST_TIMEOUT_MS}ms`);
            return;
        }

        const message = error instanceof Error ? error.message : String(error);
        await Actor.fail(message);
    }
});

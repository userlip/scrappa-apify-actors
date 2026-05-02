import { Actor } from 'apify';

const SCRAPPA_API_URL = 'https://scrappa.co/api/instagram/user/posts';
const REQUEST_TIMEOUT_MS = 90000;

function normalizeUsername(username) {
    return username.trim().replace(/^@+/, '');
}

function normalizeMaxId(maxId) {
    return typeof maxId === 'string' ? maxId.trim() : '';
}

function postsFromResponse(response) {
    if (Array.isArray(response?.posts)) {
        return response.posts;
    }

    if (Array.isArray(response?.data?.posts)) {
        return response.data.posts;
    }

    if (Array.isArray(response?.data)) {
        return response.data;
    }

    return [];
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
        return { data: { message: `HTTP ${status}` }, isJson: false };
    }

    try {
        return { data: JSON.parse(body), isJson: true };
    } catch {
        return { data: { message: body }, isJson: false };
    }
}

Actor.main(async () => {
    const input = await Actor.getInput();
    const username = typeof input?.username === 'string' ? normalizeUsername(input.username) : '';
    const maxId = normalizeMaxId(input?.max_id);

    if (!username) {
        await Actor.fail('Instagram username is required.');
        return;
    }

    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        await Actor.fail('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        return;
    }

    console.log(`Fetching Instagram user posts for: ${username}${maxId ? ` with max_id ${maxId}` : ''}`);

    try {
        const url = new URL(SCRAPPA_API_URL);
        url.searchParams.set('username', username);
        if (maxId) {
            url.searchParams.set('max_id', maxId);
        }

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

        const { data, isJson } = parseResponseBody(body, response.status);

        if (isAuthenticationFailure(response.status, data)) {
            await Actor.fail(`Scrappa API authentication failed: ${getResponseMessage(data)}. Check the SCRAPPA_API_KEY Actor secret.`);
            return;
        }

        if (response.status >= 500) {
            await Actor.fail(`Scrappa API returned HTTP ${response.status}: ${getResponseMessage(data)}`);
            return;
        }

        if (!isJson) {
            await Actor.fail(`Scrappa API returned a non-JSON response: ${getResponseMessage(data)}`);
            return;
        }

        if (response.status >= 400 || data?.success === false) {
            const statusPrefix = response.status >= 400
                ? `HTTP ${response.status}`
                : 'an error response';
            await Actor.fail(`Scrappa API returned ${statusPrefix}: ${getResponseMessage(data)}`);
            return;
        }

        const posts = postsFromResponse(data);
        const metadata = {
            request_username: username,
        };

        if (posts.length === 0) {
            console.log(`No Instagram posts returned for: ${username}`);
        }

        if (posts.length > 0) {
            await Actor.pushData(posts.map((post) => ({
                ...metadata,
                ...post,
            })));
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', data);

        console.log(`Successfully fetched ${posts.length} Instagram posts for: ${username}`);
    } catch (error) {
        if (error instanceof Error && error.name === 'AbortError') {
            await Actor.fail(`Scrappa API request timed out after ${REQUEST_TIMEOUT_MS}ms`);
            return;
        }

        const message = error instanceof Error ? error.message : String(error);
        await Actor.fail(message);
    }
});

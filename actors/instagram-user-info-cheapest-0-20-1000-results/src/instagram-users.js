const SCRAPPA_API_URL = 'https://scrappa.co/api/instagram/user';
const REQUEST_TIMEOUT_MS = 90000;
export const MAX_USERNAMES = 100;
export const REQUEST_CONCURRENCY = 5;

export class ScrappaAuthenticationError extends Error {}

export function normalizeUsername(username) {
    return username.trim().replace(/^@+/, '');
}

export function getUsernames(input) {
    const candidates = [
        ...(Array.isArray(input?.usernames) ? input.usernames : []),
        ...(typeof input?.username === 'string' ? [input.username] : []),
    ];
    const usernames = [];
    const seen = new Set();

    for (const candidate of candidates) {
        if (typeof candidate !== 'string') {
            throw new Error('Each Instagram username must be a string.');
        }

        const username = normalizeUsername(candidate);
        if (!/^[a-zA-Z0-9._-]{1,30}$/.test(username)) {
            throw new Error(`Invalid Instagram username: ${JSON.stringify(candidate)}`);
        }

        const key = username.toLowerCase();
        if (!seen.has(key)) {
            seen.add(key);
            usernames.push(username);
        }
    }

    if (usernames.length === 0) {
        throw new Error('At least one Instagram username is required. Provide usernames (recommended) or username.');
    }

    if (usernames.length > MAX_USERNAMES) {
        throw new Error(`A maximum of ${MAX_USERNAMES} unique Instagram usernames can be processed per run.`);
    }

    return usernames;
}

export function flattenProfile(response) {
    const user = response?.user ?? response?.data?.user ?? response?.data ?? response;

    if (!user || typeof user !== 'object' || Array.isArray(user)) {
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

export async function fetchInstagramUser(username, apiKey, fetchImplementation = fetch) {
    const url = new URL(SCRAPPA_API_URL);
    url.searchParams.set('username', username);

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);

    try {
        const response = await fetchImplementation(url, {
            method: 'GET',
            headers: {
                'X-API-KEY': apiKey,
                Accept: 'application/json',
            },
            signal: controller.signal,
        });
        const data = parseResponseBody(await response.text(), response.status);

        if (isAuthenticationFailure(response.status, data)) {
            throw new ScrappaAuthenticationError(`Scrappa API authentication failed: ${getResponseMessage(data)}. Check the SCRAPPA_API_KEY Actor secret.`);
        }

        if (response.status >= 400 || data?.success === false) {
            const statusPrefix = response.status >= 400
                ? `HTTP ${response.status}`
                : 'an error response';
            throw new Error(`Scrappa API returned ${statusPrefix}: ${getResponseMessage(data)}`);
        }

        return {
            ...flattenProfile(data),
            input_username: username,
        };
    } catch (error) {
        if (error instanceof Error && error.name === 'AbortError') {
            throw new Error(`Scrappa API request timed out after ${REQUEST_TIMEOUT_MS}ms`);
        }

        throw error;
    } finally {
        clearTimeout(timeoutId);
    }
}

function failureItem(username, error) {
    return {
        success: false,
        input_username: username,
        username,
        error: error instanceof Error ? error.message : String(error),
    };
}

export async function runInstagramUserBatch(usernames, fetchUser, pushData) {
    let succeeded = 0;
    let failed = 0;

    for (let offset = 0; offset < usernames.length; offset += REQUEST_CONCURRENCY) {
        const batch = usernames.slice(offset, offset + REQUEST_CONCURRENCY);
        const items = await Promise.all(batch.map(async (username) => {
            try {
                const item = await fetchUser(username);
                succeeded += 1;
                return item;
            } catch (error) {
                if (error instanceof ScrappaAuthenticationError) {
                    throw error;
                }

                failed += 1;
                console.warn(`Instagram user lookup failed for ${username}: ${error instanceof Error ? error.message : String(error)}`);
                return failureItem(username, error);
            }
        }));

        await pushData(items);
    }

    return { requested: usernames.length, succeeded, failed };
}

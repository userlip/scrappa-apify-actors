import { getResponseMessage, isTransientScrappaError, REQUEST_TIMEOUT_MS } from './retry.js';

function successfulResponse(response) {
    if (response.data?.success !== true) {
        const error = new Error(`Scrappa Instagram API: ${getResponseMessage(response.data)}`);
        error.response = { status: response.data?.status_code, data: response.data };
        throw error;
    }
    return response;
}

export function getPostIdentity(url) {
    if (typeof url !== 'string') return null;
    try {
        const parsed = new URL(/^https?:\/\//i.test(url) ? url : `https://${url}`);
        if (!['instagram.com', 'www.instagram.com'].includes(parsed.hostname)) return null;
        const match = parsed.pathname.match(/^\/([\w.]+)\/(?:p|reel|reels|tv)\/([\w-]+)\/?$/);
        return match ? { username: match[1], shortcode: match[2] } : null;
    } catch {
        return null;
    }
}

export async function fetchPost(get, params, apiKey, onFallback = () => {}) {
    // Both endpoints share one deadline so a fallback cannot double each retry's runtime.
    const options = {
        headers: { 'X-API-Key': apiKey, Accept: 'application/json' },
        timeout: REQUEST_TIMEOUT_MS,
        signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
    };
    try {
        return successfulResponse(await get('https://scrappa.co/api/instagram/post', { ...options, params }));
    } catch (error) {
        const identity = getPostIdentity(params.url);
        if (!identity || options.signal.aborted || !isTransientScrappaError(error)) throw error;

        onFallback();
        const feed = successfulResponse(await get('https://scrappa.co/api/instagram/user/posts', {
            ...options,
            params: { username: identity.username },
        }));
        const post = feed.data.posts?.find((item) => item.shortcode === identity.shortcode);
        // A missing recent-feed match does not prove that the requested post was deleted.
        if (!post) throw error;

        return { data: { success: true, found: true, data: post } };
    }
}

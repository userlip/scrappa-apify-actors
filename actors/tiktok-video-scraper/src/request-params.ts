export interface TikTokVideoInput {
    url?: unknown;
    urls?: unknown;
    hd?: unknown;
}

export interface TikTokVideoLookup {
    url: string;
    validationError?: string;
}

export function requireTikTokVideoLookup(value: string): void {
    const lookup = value.trim();
    if (lookup === '') {
        throw new Error('TikTok video URL is required');
    }

    if (/^\d{5,30}$/.test(lookup)) {
        return;
    }

    let parsed: URL;
    try {
        parsed = new URL(lookup);
    } catch {
        throw new Error('A valid TikTok video URL, short URL, photo URL, or video ID is required');
    }

    if (!/(^|\.)tiktok\.com$/i.test(parsed.hostname)) {
        throw new Error('A TikTok URL is required');
    }

    if (parsed.protocol !== 'https:') {
        throw new Error('A TikTok URL must use HTTPS');
    }

    if (/^\/@[^/]+\/video\/\d+\/?$/i.test(parsed.pathname)
        || /^\/@[^/]+\/photo\/\d+\/?$/i.test(parsed.pathname)
        || /^\/t\/[A-Za-z0-9]{6,}\/?$/i.test(parsed.pathname)) {
        return;
    }

    if (/^(vm|vt)\.tiktok\.com$/i.test(parsed.hostname)
        && /^\/[A-Za-z0-9]{8,}\/?$/i.test(parsed.pathname)) {
        return;
    }

    throw new Error('A TikTok video URL, short URL, photo URL, or video ID is required');
}

export function formatTikTokVideoLookupForLog(value: string): string {
    const lookup = value.trim();
    if (/^\d{5,30}$/.test(lookup)) {
        return `video_id:${lookup}`;
    }

    const parsed = new URL(lookup);
    parsed.search = '';
    parsed.hash = '';
    return parsed.toString();
}

export function resolveTikTokVideoRequests(
    input: TikTokVideoInput,
    warn: (message: string) => void = console.warn,
): TikTokVideoLookup[] {
    const lookups: TikTokVideoLookup[] = [];

    if (Array.isArray(input.urls)) {
        for (const [index, value] of input.urls.entries()) {
            if (typeof value !== 'string') {
                warn(`urls[${index}] must be a string, got ${typeof value}. Skipping.`);
                continue;
            }

            const lookup = value.trim();
            if (lookup === '') {
                warn(`urls[${index}] is empty. Skipping.`);
                continue;
            }

            try {
                requireTikTokVideoLookup(lookup);
                lookups.push({ url: lookup });
            } catch (error) {
                const message = error instanceof Error ? error.message : String(error);
                warn(`urls[${index}] is invalid: ${message}`);
                lookups.push({ url: lookup, validationError: message });
            }
        }
    } else if (input.urls !== undefined && input.urls !== null) {
        warn(`urls must be an array of strings, got ${typeof input.urls}. Falling back to url.`);
    }

    if (lookups.length === 0 && typeof input.url === 'string') {
        const lookup = input.url.trim();
        if (lookup !== '') {
            requireTikTokVideoLookup(lookup);
            lookups.push({ url: lookup });
        }
    } else if (lookups.length === 0 && input.url !== undefined && input.url !== null && input.url !== '') {
        warn(`url must be a string, got ${typeof input.url}.`);
    }

    if (lookups.length === 0) {
        throw new Error('At least one TikTok video URL is required');
    }

    return lookups;
}

export function resolveTikTokVideoLookups(
    input: TikTokVideoInput,
    warn: (message: string) => void = console.warn,
): string[] {
    return resolveTikTokVideoRequests(input, warn)
        .filter((lookup) => lookup.validationError === undefined)
        .map((lookup) => lookup.url);
}

export function buildTikTokVideoParams(
    url: string,
    input: TikTokVideoInput,
    warn: (message: string) => void = console.warn,
): Record<string, unknown> {
    const params: Record<string, unknown> = { url };

    if (input.hd === true) {
        params.hd = 1;
    } else if (input.hd === false || input.hd === undefined || input.hd === null || input.hd === '') {
        // Omit false so default and non-HD requests stay minimal.
    } else {
        warn(`hd must be a boolean, got ${typeof input.hd}. Using Scrappa default.`);
    }

    return params;
}

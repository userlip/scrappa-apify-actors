export interface TikTokAdsInput {
    url?: unknown;
    urls?: unknown;
}

export interface TikTokAdLookup {
    url: string;
    validationError?: string;
}

export function requireTikTokAdUrl(value: string): void {
    const lookup = value.trim();
    if (lookup === '') {
        throw new Error('TikTok Creative Center ad URL is required');
    }

    let parsed: URL;
    try {
        parsed = new URL(lookup);
    } catch {
        throw new Error('A valid TikTok Creative Center ad URL is required');
    }

    if (!/(^|\.)tiktok\.com$/i.test(parsed.hostname)) {
        throw new Error('A TikTok Creative Center ad URL on ads.tiktok.com is required');
    }

    if (!/^ads\.tiktok\.com$/i.test(parsed.hostname)) {
        throw new Error('A TikTok Creative Center ad URL on ads.tiktok.com is required');
    }

    if (parsed.protocol !== 'https:') {
        throw new Error('TikTok Creative Center ad URLs must use HTTPS');
    }

    if (/^\/business\/creativecenter\/topads\/\d+(?:\/pc\/en)?\/?$/i.test(parsed.pathname)) {
        return;
    }

    throw new Error('TikTok Creative Center ad URLs must use the format https://ads.tiktok.com/business/creativecenter/topads/{ad_id}/pc/en');
}

export function normalizeTikTokAdUrl(value: string): string {
    requireTikTokAdUrl(value);

    const parsed = new URL(value.trim());
    parsed.search = '';
    parsed.hash = '';

    return parsed.toString();
}

export function extractTikTokAdId(value: string): string | null {
    try {
        const parsed = new URL(value.trim());
        const match = parsed.pathname.match(/^\/business\/creativecenter\/topads\/(\d+)(?:\/pc\/en)?\/?$/i);
        return match?.[1] ?? null;
    } catch {
        return null;
    }
}

export function formatTikTokAdLookupForLog(value: string): string {
    const normalized = normalizeTikTokAdUrl(value);
    const adId = extractTikTokAdId(normalized);

    return adId ? `ad_id:${adId}` : normalized;
}

export function resolveTikTokAdRequests(
    input: TikTokAdsInput,
    warn: (message: string) => void = console.warn,
): TikTokAdLookup[] {
    const lookups: TikTokAdLookup[] = [];

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
                lookups.push({ url: normalizeTikTokAdUrl(lookup) });
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
            lookups.push({ url: normalizeTikTokAdUrl(lookup) });
        }
    } else if (lookups.length === 0 && input.url !== undefined && input.url !== null && input.url !== '') {
        warn(`url must be a string, got ${typeof input.url}.`);
    }

    if (lookups.length === 0) {
        throw new Error('At least one TikTok Creative Center ad URL is required');
    }

    return lookups;
}

export function resolveTikTokAdUrls(
    input: TikTokAdsInput,
    warn: (message: string) => void = console.warn,
): string[] {
    return resolveTikTokAdRequests(input, warn)
        .filter((lookup) => lookup.validationError === undefined)
        .map((lookup) => lookup.url);
}

export function buildTikTokAdParams(url: string): Record<string, unknown> {
    return { url };
}

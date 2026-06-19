export interface TikTokChallengeSearchInput {
    keywords?: unknown;
    keyword?: unknown;
    count?: unknown;
}

export interface TikTokChallengeSearchRequest {
    keyword: string;
    params: Record<string, unknown>;
}

export function formatTikTokChallengeSearchLookupForLog(requests: TikTokChallengeSearchRequest[]): string {
    if (requests.length === 0) {
        return 'unknown TikTok challenge search';
    }

    if (requests.length === 1) {
        return requests[0].keyword;
    }

    return `${requests.length} TikTok challenge searches`;
}

export function normalizeTikTokChallengeSearchKeyword(value: string): string {
    const keyword = value.trim().replace(/\s+/g, ' ');

    if (keyword === '') {
        return '';
    }

    if (keyword.length > 255) {
        throw new Error('TikTok challenge search keywords must be 255 characters or fewer');
    }

    if (/[\r\n\t\f\v]/.test(value)) {
        throw new Error('TikTok challenge search keywords cannot contain tabs, line breaks, or control whitespace');
    }

    return keyword;
}

function collectKeywords(input: TikTokChallengeSearchInput, warn: (message: string) => void): string[] {
    const keywords: string[] = [];

    if (Array.isArray(input.keywords)) {
        for (const value of input.keywords) {
            if (typeof value === 'string') {
                const keyword = normalizeTikTokChallengeSearchKeyword(value);
                if (keyword !== '') {
                    keywords.push(keyword);
                }
            } else if (value !== undefined && value !== null) {
                warn(`keywords entries must be strings, got ${typeof value}. Omitting entry.`);
            }
        }
    } else if (typeof input.keywords === 'string') {
        const keyword = normalizeTikTokChallengeSearchKeyword(input.keywords);
        if (keyword !== '') {
            keywords.push(keyword);
        }
    } else if (input.keywords !== undefined && input.keywords !== null) {
        warn(`keywords must be an array of strings, got ${typeof input.keywords}. Falling back to keyword.`);
    }

    if (keywords.length > 0) {
        return [...new Set(keywords)];
    }

    if (typeof input.keyword === 'string') {
        const keyword = normalizeTikTokChallengeSearchKeyword(input.keyword);
        if (keyword !== '') {
            keywords.push(keyword);
        }
    } else if (input.keyword !== undefined && input.keyword !== null) {
        warn(`keyword must be a string, got ${typeof input.keyword}.`);
    }

    return [...new Set(keywords)];
}

function normalizeCount(value: unknown, warn: (message: string) => void): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value === 'number' && Number.isInteger(value) && value >= 1 && value <= 50) {
        return value;
    }

    warn(`count must be an integer between 1 and 50, got ${String(value)}. Using Scrappa default.`);
    return undefined;
}

export function buildTikTokChallengeSearchRequests(
    input: TikTokChallengeSearchInput,
    warn: (message: string) => void = console.warn,
): TikTokChallengeSearchRequest[] {
    const keywords = collectKeywords(input, warn);

    if (keywords.length === 0) {
        throw new Error('At least one TikTok challenge search keyword is required');
    }

    const count = normalizeCount(input.count, warn);

    return keywords.map((keyword) => ({
        keyword,
        params: {
            keywords: keyword,
            ...(count !== undefined ? { count } : {}),
        },
    }));
}

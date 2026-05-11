export interface TikTokHashtagPostsInput {
    hashtag?: unknown;
    challenge_name?: unknown;
    challenge_id?: unknown;
    region?: unknown;
    count?: unknown;
    cursor?: unknown;
}

type TikTokChallengeLookup = {
    key: 'challenge_id' | 'challenge_name';
    value: string;
    logValue: string;
};

export function formatTikTokHashtagPostsLookupForLog(input: TikTokHashtagPostsInput): string {
    return resolveTikTokChallengeLookup(input)?.logValue ?? 'unknown TikTok hashtag';
}

export function normalizeTikTokHashtag(value: string): string {
    const trimmed = value.trim();

    if (trimmed === '') {
        return '';
    }

    const isUrlLike = /^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed)
        || trimmed.startsWith('//');

    if (isUrlLike) {
        let parsed: URL;
        try {
            parsed = new URL(trimmed);
        } catch {
            throw new Error('A valid TikTok hashtag URL or hashtag name is required');
        }

        if (!/(^|\.)tiktok\.com$/i.test(parsed.hostname)) {
            throw new Error('TikTok hashtag URL must be on tiktok.com');
        }
        if (parsed.protocol !== 'https:') {
            throw new Error('TikTok hashtag URL must use HTTPS');
        }

        const match = parsed.pathname.match(/^\/tag\/([^/?#]+)\/?$/i);
        if (!match) {
            throw new Error('TikTok hashtag URL must use the format https://www.tiktok.com/tag/hashtag');
        }

        return normalizeHashtagName(decodeURIComponent(match[1]));
    }

    return normalizeHashtagName(trimmed);
}

function normalizeHashtagName(value: string): string {
    const hashtag = value.startsWith('#') ? value.slice(1) : value;
    if (!/^[^\s?#/=:]{1,255}$/u.test(hashtag)) {
        throw new Error('TikTok hashtag must be 1 to 255 characters and cannot contain whitespace or URL delimiter characters');
    }

    return hashtag;
}

function normalizeTikTokChallengeLookup(value: string): TikTokChallengeLookup {
    const trimmed = value.trim();
    if (/^\d+$/.test(trimmed)) {
        const challengeId = normalizeTikTokChallengeId(trimmed);
        return { key: 'challenge_id', value: challengeId, logValue: `challenge_id:${challengeId}` };
    }

    const challengeName = normalizeTikTokHashtag(trimmed);
    return { key: 'challenge_name', value: challengeName, logValue: `#${challengeName}` };
}

function normalizeTikTokChallengeId(value: string): string {
    const trimmed = value.trim();
    if (trimmed === '') {
        return '';
    }
    if (!/^\d+$/.test(trimmed)) {
        throw new Error('TikTok challenge_id must contain digits only');
    }
    if (trimmed.length > 100) {
        throw new Error('TikTok challenge_id must be 100 digits or fewer');
    }
    return trimmed;
}

function resolveTikTokChallengeLookup(
    input: TikTokHashtagPostsInput,
    warn?: (message: string) => void,
): TikTokChallengeLookup | null {
    if (typeof input.hashtag === 'string') {
        const hashtag = input.hashtag.trim();
        if (hashtag !== '') {
            return normalizeTikTokChallengeLookup(hashtag);
        }
    } else if (input.hashtag !== undefined && input.hashtag !== null) {
        warn?.(`hashtag must be a string, got ${typeof input.hashtag}.`);
    }

    if (typeof input.challenge_name === 'string') {
        const challengeName = normalizeTikTokHashtag(input.challenge_name);
        if (challengeName !== '') {
            return { key: 'challenge_name', value: challengeName, logValue: `#${challengeName}` };
        }
    } else if (input.challenge_name !== undefined && input.challenge_name !== null) {
        warn?.(`challenge_name must be a string, got ${typeof input.challenge_name}.`);
    }

    if (typeof input.challenge_id === 'string') {
        const challengeId = normalizeTikTokChallengeId(input.challenge_id);
        if (challengeId !== '') {
            return { key: 'challenge_id', value: challengeId, logValue: `challenge_id:${challengeId}` };
        }
    } else if (input.challenge_id !== undefined && input.challenge_id !== null) {
        warn?.(`challenge_id must be a string, got ${typeof input.challenge_id}.`);
    }

    return null;
}

export function buildTikTokHashtagPostsParams(
    input: TikTokHashtagPostsInput,
    warn: (message: string) => void = console.warn,
): Record<string, unknown> {
    const params: Record<string, unknown> = {};

    const lookup = resolveTikTokChallengeLookup(input, warn);
    if (lookup) {
        params[lookup.key] = lookup.value;
    }

    if (typeof input.region === 'string') {
        const region = input.region.trim().toUpperCase();
        if (region !== '') {
            if (/^[A-Z]{2,10}$/.test(region)) {
                params.region = region;
            } else {
                warn('region must be a 2 to 10 character country or region code. Omitting region.');
            }
        }
    } else if (input.region !== undefined && input.region !== null) {
        warn(`region must be a string, got ${typeof input.region}. Omitting region.`);
    }

    if (input.count !== undefined) {
        if (
            typeof input.count === 'number'
            && Number.isInteger(input.count)
            && input.count >= 1
            && input.count <= 50
        ) {
            params.count = input.count;
        } else {
            warn(`count must be an integer between 1 and 50, got ${String(input.count)}. Using Scrappa default.`);
        }
    }

    if (typeof input.cursor === 'string') {
        const cursor = input.cursor.trim();
        if (cursor !== '') {
            params.cursor = cursor;
        }
    } else if (typeof input.cursor === 'number') {
        if (Number.isSafeInteger(input.cursor)) {
            params.cursor = String(input.cursor);
        } else {
            warn(`cursor must be a string or safe integer, got ${String(input.cursor)}. Starting from the first page.`);
        }
    } else if (input.cursor !== undefined && input.cursor !== null) {
        warn(`cursor must be a string or number, got ${typeof input.cursor}. Starting from the first page.`);
    }

    if (!lookup) {
        throw new Error('TikTok challenge_id or challenge_name is required');
    }

    return params;
}

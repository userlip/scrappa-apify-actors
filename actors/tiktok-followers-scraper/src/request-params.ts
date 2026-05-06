export interface TikTokFollowersInput {
    profile?: unknown;
    unique_id?: unknown;
    user_id?: unknown;
    count?: unknown;
    time?: unknown;
    cursor?: unknown;
}

type TikTokLookup = {
    key: 'unique_id' | 'user_id';
    value: string;
    logValue: string;
};

export function formatTikTokFollowersLookupForLog(input: TikTokFollowersInput): string {
    return resolveTikTokFollowersLookup(input)?.logValue ?? 'unknown TikTok profile';
}

export function normalizeTikTokUniqueId(value: string): string {
    const trimmed = value.trim();

    if (trimmed === '') {
        return '';
    }

    const isUrlLike = /^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed)
        || trimmed.startsWith('//')
        || /(^|\.)tiktok\.com(\/|$)/i.test(trimmed);

    try {
        const parsed = new URL(trimmed);
        if (!/(^|\.)tiktok\.com$/i.test(parsed.hostname)) {
            throw new Error('TikTok profile URL must be on tiktok.com');
        }
        if (parsed.protocol !== 'https:') {
            throw new Error('TikTok profile URL must use HTTPS');
        }

        const match = parsed.pathname.match(/^\/(@[^/?#]+)\/?$/i);
        if (!match) {
            throw new Error('TikTok profile URL must use the format https://www.tiktok.com/@username');
        }

        return normalizeTikTokUsername(match[1]);
    } catch (error) {
        if (error instanceof Error && (error.message.startsWith('TikTok profile URL') || error.message.startsWith('TikTok username'))) {
            throw error;
        }
    }

    if (isUrlLike) {
        throw new Error('A valid TikTok profile URL or username is required');
    }

    return normalizeTikTokUsername(trimmed);
}

function normalizeTikTokUsername(value: string): string {
    const username = value.startsWith('@') ? value.slice(1) : value;
    if (!/^[A-Za-z0-9._]{2,255}$/.test(username)) {
        throw new Error('TikTok username must be 2 to 255 characters and contain only letters, numbers, dots, or underscores');
    }

    return `@${username}`;
}

function normalizeTikTokProfileLookup(value: string): TikTokLookup {
    const trimmed = value.trim();
    if (/^\d+$/.test(trimmed)) {
        const userId = normalizeTikTokUserId(trimmed);
        return { key: 'user_id', value: userId, logValue: `user_id:${userId}` };
    }

    const uniqueId = normalizeTikTokUniqueId(trimmed);
    return { key: 'unique_id', value: uniqueId, logValue: uniqueId };
}

function resolveTikTokFollowersLookup(
    input: TikTokFollowersInput,
    warn?: (message: string) => void,
): TikTokLookup | null {
    if (typeof input.profile === 'string') {
        const profile = input.profile.trim();
        if (profile !== '') {
            return normalizeTikTokProfileLookup(profile);
        }
    } else if (input.profile !== undefined && input.profile !== null && input.profile !== '') {
        warn?.(`profile must be a string, got ${typeof input.profile}.`);
    }

    if (typeof input.unique_id === 'string') {
        const uniqueId = normalizeTikTokUniqueId(input.unique_id);
        if (uniqueId !== '') {
            return { key: 'unique_id', value: uniqueId, logValue: uniqueId };
        }
    } else if (input.unique_id !== undefined && input.unique_id !== null && input.unique_id !== '') {
        warn?.(`unique_id must be a string, got ${typeof input.unique_id}.`);
    }

    if (typeof input.user_id === 'string') {
        const userId = normalizeTikTokUserId(input.user_id);
        if (userId !== '') {
            return { key: 'user_id', value: userId, logValue: `user_id:${userId}` };
        }
    } else if (input.user_id !== undefined && input.user_id !== null && input.user_id !== '') {
        warn?.(`user_id must be a string, got ${typeof input.user_id}.`);
    }

    return null;
}

function normalizeTikTokUserId(value: string): string {
    const trimmed = value.trim();
    if (trimmed === '') {
        return '';
    }
    if (!/^\d+$/.test(trimmed)) {
        throw new Error('TikTok user_id must contain digits only');
    }
    if (trimmed.length > 30) {
        throw new Error('TikTok numeric user ID must be 30 digits or fewer');
    }
    return trimmed;
}

export function buildTikTokFollowersParams(
    input: TikTokFollowersInput,
    warn: (message: string) => void = console.warn,
): Record<string, unknown> {
    const params: Record<string, unknown> = {};

    const lookup = resolveTikTokFollowersLookup(input, warn);
    if (lookup) {
        params[lookup.key] = lookup.value;
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

    const paginationValue = input.time ?? input.cursor;
    const paginationField = input.time !== undefined ? 'time' : 'cursor';

    if (typeof paginationValue === 'number') {
        if (Number.isInteger(paginationValue) && paginationValue >= 0) {
            params.time = paginationValue;
        } else {
            warn(`${paginationField} must be a non-negative integer, got ${String(paginationValue)}. Starting from the first page.`);
        }
    } else if (typeof paginationValue === 'string') {
        const time = paginationValue.trim();
        if (time !== '') {
            if (/^\d+$/.test(time)) {
                params.time = time;
            } else {
                warn(`${paginationField} must contain digits only. Starting from the first page.`);
            }
        }
    } else if (paginationValue !== undefined && paginationValue !== null && paginationValue !== '') {
        warn(`${paginationField} must be a non-negative integer or digit string, got ${typeof paginationValue}. Starting from the first page.`);
    }

    if (!lookup) {
        throw new Error('TikTok unique_id or user_id is required');
    }

    return params;
}

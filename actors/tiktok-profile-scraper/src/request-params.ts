export interface TikTokProfileInput {
    profile?: unknown;
    unique_id?: unknown;
    user_id?: unknown;
}

export function formatTikTokProfileLookupForLog(input: TikTokProfileInput): string {
    const profile = typeof input.profile === 'string' ? input.profile.trim() : '';
    const uniqueId = typeof input.unique_id === 'string' ? input.unique_id.trim() : '';
    const userId = typeof input.user_id === 'string' ? input.user_id.trim() : '';

    if (profile !== '') {
        return normalizeTikTokProfileLookup(profile).logValue;
    }

    if (uniqueId !== '') {
        return normalizeTikTokUniqueId(uniqueId);
    }

    if (userId !== '') {
        return `user_id:${userId}`;
    }

    return 'unknown TikTok profile';
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

function normalizeTikTokProfileLookup(value: string): { key: 'unique_id' | 'user_id'; value: string; logValue: string } {
    const trimmed = value.trim();
    if (/^\d+$/.test(trimmed)) {
        const userId = normalizeTikTokUserId(trimmed);
        return { key: 'user_id', value: userId, logValue: `user_id:${userId}` };
    }

    const uniqueId = normalizeTikTokUniqueId(trimmed);
    return { key: 'unique_id', value: uniqueId, logValue: uniqueId };
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

export function buildTikTokProfileParams(
    input: TikTokProfileInput,
    warn: (message: string) => void = console.warn,
): Record<string, unknown> {
    const params: Record<string, unknown> = {};

    if (typeof input.profile === 'string') {
        const profile = input.profile.trim();
        if (profile !== '') {
            const lookup = normalizeTikTokProfileLookup(profile);
            params[lookup.key] = lookup.value;
        }
    } else if (input.profile !== undefined && input.profile !== null && input.profile !== '') {
        warn(`profile must be a string, got ${typeof input.profile}.`);
    }

    const hasProfileLookup = Boolean(params.unique_id || params.user_id);

    if (!hasProfileLookup && typeof input.unique_id === 'string') {
        const uniqueId = normalizeTikTokUniqueId(input.unique_id);
        if (uniqueId !== '') {
            params.unique_id = uniqueId;
        }
    } else if (!hasProfileLookup && input.unique_id !== undefined && input.unique_id !== null && input.unique_id !== '') {
        warn(`unique_id must be a string, got ${typeof input.unique_id}.`);
    }

    if (!hasProfileLookup && typeof input.user_id === 'string') {
        const userId = normalizeTikTokUserId(input.user_id);
        if (userId !== '') {
            params.user_id = userId;
        }
    } else if (!hasProfileLookup && input.user_id !== undefined && input.user_id !== null && input.user_id !== '') {
        warn(`user_id must be a string, got ${typeof input.user_id}.`);
    }

    if (!params.unique_id && !params.user_id) {
        throw new Error('TikTok unique_id or user_id is required');
    }

    return params;
}

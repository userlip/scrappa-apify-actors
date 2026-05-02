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

        return match[1];
    } catch (error) {
        if (error instanceof Error && error.message.startsWith('TikTok profile URL')) {
            throw error;
        }
    }

    if (isUrlLike) {
        throw new Error('A valid TikTok profile URL or username is required');
    }

    return trimmed.startsWith('@') ? trimmed : `@${trimmed}`;
}

function normalizeTikTokProfileLookup(value: string): { key: 'unique_id' | 'user_id'; value: string; logValue: string } {
    const trimmed = value.trim();
    if (/^\d{1,30}$/.test(trimmed)) {
        return { key: 'user_id', value: trimmed, logValue: `user_id:${trimmed}` };
    }

    const uniqueId = normalizeTikTokUniqueId(trimmed);
    return { key: 'unique_id', value: uniqueId, logValue: uniqueId };
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

    if (typeof input.unique_id === 'string') {
        const uniqueId = normalizeTikTokUniqueId(input.unique_id);
        if (uniqueId !== '') {
            params.unique_id = uniqueId;
        }
    } else if (input.unique_id !== undefined && input.unique_id !== null && input.unique_id !== '') {
        warn(`unique_id must be a string, got ${typeof input.unique_id}.`);
    }

    if (typeof input.user_id === 'string') {
        const userId = input.user_id.trim();
        if (userId !== '') {
            params.user_id = userId;
        }
    } else if (input.user_id !== undefined && input.user_id !== null && input.user_id !== '') {
        warn(`user_id must be a string, got ${typeof input.user_id}.`);
    }

    if (!params.unique_id && !params.user_id) {
        throw new Error('TikTok unique_id or user_id is required');
    }

    return params;
}

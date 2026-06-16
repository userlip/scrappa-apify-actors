export interface TikTokMusicPostsInput {
    musicIds?: unknown;
    music_id?: unknown;
    count?: unknown;
    cursor?: unknown;
}

export interface TikTokMusicPostsRequest {
    musicId: string;
    params: Record<string, unknown>;
}

export function formatTikTokMusicPostsLookupForLog(requests: TikTokMusicPostsRequest[]): string {
    if (requests.length === 0) {
        return 'unknown TikTok music';
    }

    if (requests.length === 1) {
        return `music_id:${requests[0].musicId}`;
    }

    return `${requests.length} TikTok music IDs`;
}

export function normalizeTikTokMusicId(value: string): string {
    const trimmed = value.trim();
    if (trimmed === '') {
        return '';
    }

    if (!/^\d+$/.test(trimmed)) {
        throw new Error('TikTok music_id must contain digits only');
    }

    if (trimmed.length > 100) {
        throw new Error('TikTok music_id must be 100 digits or fewer');
    }

    return trimmed;
}

function collectMusicIds(input: TikTokMusicPostsInput, warn: (message: string) => void): string[] {
    const musicIds: string[] = [];

    if (Array.isArray(input.musicIds)) {
        for (const value of input.musicIds) {
            if (typeof value === 'string') {
                const musicId = normalizeTikTokMusicId(value);
                if (musicId !== '') {
                    musicIds.push(musicId);
                }
            } else if (typeof value === 'number' && Number.isSafeInteger(value)) {
                musicIds.push(normalizeTikTokMusicId(String(value)));
            } else if (value !== undefined && value !== null) {
                warn(`musicIds entries must be strings or safe integers, got ${typeof value}. Omitting entry.`);
            }
        }
    } else if (input.musicIds !== undefined && input.musicIds !== null) {
        warn(`musicIds must be an array, got ${typeof input.musicIds}. Falling back to music_id.`);
    }

    if (musicIds.length > 0) {
        return [...new Set(musicIds)];
    }

    if (typeof input.music_id === 'string') {
        const musicId = normalizeTikTokMusicId(input.music_id);
        if (musicId !== '') {
            musicIds.push(musicId);
        }
    } else if (typeof input.music_id === 'number' && Number.isSafeInteger(input.music_id)) {
        musicIds.push(normalizeTikTokMusicId(String(input.music_id)));
    } else if (input.music_id !== undefined && input.music_id !== null) {
        warn(`music_id must be a string or safe integer, got ${typeof input.music_id}.`);
    }

    return [...new Set(musicIds)];
}

export function buildTikTokMusicPostsRequests(
    input: TikTokMusicPostsInput,
    warn: (message: string) => void = console.warn,
): TikTokMusicPostsRequest[] {
    const musicIds = collectMusicIds(input, warn);

    if (musicIds.length === 0) {
        throw new Error('At least one TikTok music_id is required');
    }

    const sharedParams: Record<string, unknown> = {};

    if (input.count !== undefined) {
        if (
            typeof input.count === 'number'
            && Number.isInteger(input.count)
            && input.count >= 1
            && input.count <= 50
        ) {
            sharedParams.count = input.count;
        } else {
            warn(`count must be an integer between 1 and 50, got ${String(input.count)}. Using Scrappa default.`);
        }
    }

    if (typeof input.cursor === 'string') {
        const cursor = input.cursor.trim();
        if (cursor !== '') {
            sharedParams.cursor = cursor;
        }
    } else if (typeof input.cursor === 'number') {
        if (Number.isSafeInteger(input.cursor)) {
            sharedParams.cursor = String(input.cursor);
        } else {
            warn(`cursor must be a string or safe integer, got ${String(input.cursor)}. Starting from the first page.`);
        }
    } else if (input.cursor !== undefined && input.cursor !== null) {
        warn(`cursor must be a string or number, got ${typeof input.cursor}. Starting from the first page.`);
    }

    return musicIds.map((musicId) => ({
        musicId,
        params: {
            ...sharedParams,
            music_id: musicId,
        },
    }));
}

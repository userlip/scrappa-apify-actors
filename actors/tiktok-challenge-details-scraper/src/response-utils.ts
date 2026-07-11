export interface TikTokChallengeDetail {
    id?: string | number; challenge_id?: string | number; cid?: string | number;
    challenge_name?: string; cha_name?: string; name?: string; title?: string;
    description?: string; desc?: string; cover?: string; cover_url?: string;
    view_count?: number; video_count?: number; user_count?: number; stats?: Record<string, unknown>;
    [key: string]: unknown;
}

export interface TikTokChallengeDetailsResponse { code?: number; msg?: string; data?: TikTokChallengeDetail | { challenge?: TikTokChallengeDetail; item?: TikTokChallengeDetail } | null; [key: string]: unknown; }

function text(value: unknown): string | null { return typeof value === 'string' && value.trim() ? value.trim() : typeof value === 'number' && Number.isSafeInteger(value) ? String(value) : null; }
function count(value: unknown): number | null { return typeof value === 'number' ? value : null; }

export function challengeName(challenge: TikTokChallengeDetail): string | null {
    return text(challenge.challenge_name ?? challenge.cha_name ?? challenge.name ?? challenge.title);
}

export function extractChallengeDetail(data: TikTokChallengeDetailsResponse['data']): TikTokChallengeDetail | null {
    if (!data || Array.isArray(data) || typeof data !== 'object') return null;
    if ('challenge' in data && data.challenge && typeof data.challenge === 'object') return data.challenge as TikTokChallengeDetail;
    if ('item' in data && data.item && typeof data.item === 'object') return data.item as TikTokChallengeDetail;
    return data as TikTokChallengeDetail;
}

export function normalizeChallengeDetail(challenge: TikTokChallengeDetail, request: { type: 'challenge_name' | 'challenge_id'; value: string }, retrievedAt = new Date().toISOString()): Record<string, unknown> {
    return {
        ...challenge,
        challenge_id: text(challenge.challenge_id ?? challenge.id ?? challenge.cid),
        challenge_name: challengeName(challenge),
        description: text(challenge.description ?? challenge.desc),
        user_count: count(challenge.user_count ?? challenge.stats?.user_count),
        view_count: count(challenge.view_count ?? challenge.stats?.view_count),
        video_count: count(challenge.video_count ?? challenge.stats?.video_count),
        cover: text(challenge.cover ?? challenge.cover_url),
        request_challenge_name: request.type === 'challenge_name' ? request.value : null,
        request_challenge_id: request.type === 'challenge_id' ? request.value : null,
        retrieved_at: retrievedAt,
    };
}

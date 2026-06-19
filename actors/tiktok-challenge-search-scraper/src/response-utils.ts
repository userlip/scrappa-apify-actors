export interface TikTokChallenge {
    id?: string | number;
    challenge_id?: string | number;
    cid?: string | number;
    cha_name?: string;
    challenge_name?: string;
    name?: string;
    title?: string;
    desc?: string;
    description?: string;
    view_count?: number;
    video_count?: number;
    user_count?: number;
    stats?: Record<string, unknown>;
    [key: string]: unknown;
}

export interface TikTokChallengeSearchData {
    challenges?: TikTokChallenge[];
    challenge_list?: TikTokChallenge[];
    challengeList?: TikTokChallenge[];
    items?: TikTokChallenge[];
    results?: TikTokChallenge[];
    [key: string]: unknown;
}

export interface TikTokChallengeSearchResponse {
    code?: number;
    msg?: string;
    processed_time?: number;
    data?: TikTokChallengeSearchData | TikTokChallenge[] | null;
    [key: string]: unknown;
}

export function extractChallenges(data: TikTokChallengeSearchResponse['data']): TikTokChallenge[] {
    if (!data) {
        return [];
    }

    if (Array.isArray(data)) {
        return data;
    }

    if (Array.isArray(data.challenges)) {
        return data.challenges;
    }

    if (Array.isArray(data.challenge_list)) {
        return data.challenge_list;
    }

    if (Array.isArray(data.challengeList)) {
        return data.challengeList;
    }

    if (Array.isArray(data.items)) {
        return data.items;
    }

    if (Array.isArray(data.results)) {
        return data.results;
    }

    return [];
}

export function getChallengeId(challenge: TikTokChallenge): string | null {
    const id = challenge.challenge_id ?? challenge.id ?? challenge.cid;
    if (typeof id === 'string') {
        const trimmed = id.trim();
        return trimmed !== '' ? trimmed : null;
    }

    if (typeof id === 'number' && Number.isSafeInteger(id)) {
        return String(id);
    }

    return null;
}

export function getChallengeName(challenge: TikTokChallenge): string | null {
    const name = challenge.challenge_name ?? challenge.cha_name ?? challenge.name ?? challenge.title;
    if (typeof name !== 'string') {
        return null;
    }

    const trimmed = name.trim();
    return trimmed !== '' ? trimmed : null;
}

export function normalizeChallengeResult(
    challenge: TikTokChallenge,
    request: { keyword: string; count: unknown },
): Record<string, unknown> {
    return {
        ...challenge,
        challenge_id: getChallengeId(challenge),
        challenge_name: getChallengeName(challenge),
        description: typeof challenge.description === 'string' ? challenge.description : challenge.desc ?? null,
        view_count: challenge.view_count ?? challenge.stats?.view_count ?? null,
        video_count: challenge.video_count ?? challenge.stats?.video_count ?? null,
        user_count: challenge.user_count ?? challenge.stats?.user_count ?? null,
        request_keyword: request.keyword,
        request_count: request.count ?? null,
    };
}

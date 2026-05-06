interface TikTokProfileUser {
    id?: string | number;
    [key: string]: unknown;
}

export interface TikTokProfileRecord {
    user_id?: string | number;
    user?: TikTokProfileUser;
    [key: string]: unknown;
}

export interface TikTokProfileResponse {
    code?: number;
    msg?: string;
    data?: TikTokProfileRecord | TikTokProfileRecord[] | null;
    [key: string]: unknown;
}

export function extractProfileUserId(data: TikTokProfileResponse['data']): string | null {
    const profile = Array.isArray(data) ? data[0] : data;
    const userId = profile?.user_id ?? profile?.user?.id;

    if (userId === undefined || userId === null || userId === '') {
        return null;
    }

    return String(userId);
}

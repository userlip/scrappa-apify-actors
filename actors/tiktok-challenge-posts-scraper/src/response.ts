export interface ScrappaResponse {
    code?: number;
    msg?: string;
    processed_time?: number;
    data?: unknown;
}

export interface Page {
    videos: Record<string, unknown>[];
    cursor: string | null;
    hasMore: boolean;
}

function record(value: unknown): Record<string, unknown> | null {
    return value !== null && typeof value === 'object' && !Array.isArray(value)
        ? value as Record<string, unknown>
        : null;
}

export function parsePage(data: unknown): Page {
    const body = record(data);
    const values = Array.isArray(data)
        ? data
        : body && Array.isArray(body.posts)
            ? body.posts
            : body && Array.isArray(body.videos)
                ? body.videos
            : body && Array.isArray(body.aweme_list)
                ? body.aweme_list
                : [];
    const cursorValue = body?.cursor ?? body?.max_cursor ?? body?.min_cursor ?? null;

    return {
        videos: values.map(record).filter((video): video is Record<string, unknown> => video !== null),
        cursor: typeof cursorValue === 'string' || typeof cursorValue === 'number' ? String(cursorValue) : null,
        hasMore: body?.hasMore === true || body?.has_more === true,
    };
}

export function getVideoId(video: Record<string, unknown>): string | null {
    for (const value of [video.video_id, video.aweme_id, video.id]) {
        if (typeof value === 'string' && value !== '') {
            return value;
        }
        if (typeof value === 'number' && Number.isSafeInteger(value)) {
            return String(value);
        }
    }
    return null;
}

export interface TikTokCommentsInput {
    url: string;
    count?: unknown;
    cursor?: unknown;
    includeReplies?: unknown;
    maxRepliesPerComment?: unknown;
}

export interface TikTokCommentRepliesInput {
    comment_id: string;
    video_id?: string;
    count?: unknown;
    cursor?: unknown;
}

export function requireTikTokVideoUrl(url: string): void {
    let parsed: URL;

    try {
        parsed = new URL(url);
    } catch {
        throw new Error('A valid TikTok video URL is required');
    }

    if (!/(^|\.)tiktok\.com$/i.test(parsed.hostname)) {
        throw new Error('A TikTok video URL is required');
    }

    if (parsed.protocol !== 'https:') {
        throw new Error('A TikTok video URL must use HTTPS');
    }

    if (!/^\/@[^/]+\/video\/\d+\/?$/i.test(parsed.pathname)) {
        throw new Error('A TikTok video URL must use the format https://www.tiktok.com/@username/video/1234567890');
    }
}

export function formatTikTokVideoUrlForLog(url: string): string {
    const parsed = new URL(url);
    parsed.search = '';
    parsed.hash = '';
    return parsed.toString();
}

export function extractTikTokVideoId(url: string): string {
    const parsed = new URL(url);
    const match = parsed.pathname.match(/\/video\/(\d+)\/?$/i);
    if (!match) {
        throw new Error('A TikTok video URL must use the format https://www.tiktok.com/@username/video/1234567890');
    }
    return match[1];
}

export function buildTikTokCommentsParams(
    input: TikTokCommentsInput,
    warn: (message: string) => void = console.warn,
): Record<string, unknown> {
    const params: Record<string, unknown> = {
        url: input.url,
    };

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
    } else if (input.cursor !== undefined && input.cursor !== null && input.cursor !== '') {
        warn(`cursor must be a string, got ${typeof input.cursor}. Starting from the first page.`);
    }

    return params;
}

export function shouldIncludeTikTokCommentReplies(input: TikTokCommentsInput): boolean {
    return input.includeReplies === true;
}

export function resolveMaxRepliesPerComment(
    input: TikTokCommentsInput,
    warn: (message: string) => void = console.warn,
): number {
    if (input.maxRepliesPerComment === undefined) {
        return 50;
    }

    if (
        typeof input.maxRepliesPerComment === 'number'
        && Number.isInteger(input.maxRepliesPerComment)
        && input.maxRepliesPerComment >= 1
        && input.maxRepliesPerComment <= 500
    ) {
        return input.maxRepliesPerComment;
    }

    warn(`maxRepliesPerComment must be an integer between 1 and 500, got ${String(input.maxRepliesPerComment)}. Using 50.`);
    return 50;
}

export function buildTikTokCommentRepliesParams(
    input: TikTokCommentRepliesInput,
    warn: (message: string) => void = console.warn,
): Record<string, unknown> {
    const commentId = input.comment_id.trim();
    if (commentId === '') {
        throw new Error('comment_id is required to fetch TikTok comment replies');
    }

    const params: Record<string, unknown> = {
        comment_id: commentId,
    };

    if (typeof input.video_id === 'string' && input.video_id.trim() !== '') {
        params.video_id = input.video_id.trim();
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
            warn(`reply count must be an integer between 1 and 50, got ${String(input.count)}. Using Scrappa default.`);
        }
    }

    if (typeof input.cursor === 'string') {
        const cursor = input.cursor.trim();
        if (cursor !== '') {
            params.cursor = cursor;
        }
    } else if (input.cursor !== undefined && input.cursor !== null && input.cursor !== '') {
        warn(`reply cursor must be a string, got ${typeof input.cursor}. Starting from the first reply page.`);
    }

    return params;
}

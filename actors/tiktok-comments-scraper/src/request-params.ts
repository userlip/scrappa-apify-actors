export interface TikTokCommentsInput {
    url: string;
    count?: unknown;
    cursor?: unknown;
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

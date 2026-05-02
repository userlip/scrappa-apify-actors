import { Actor } from 'apify';
import { ScrappaClient } from './shared/scrappa-client.js';
import { buildTikTokCommentsParams, requireTikTokVideoUrl } from './request-params.js';
import type { TikTokCommentsInput } from './request-params.js';

interface TikTokCommentUser {
    user_id?: string;
    unique_id?: string;
    nickname?: string;
    avatar?: string;
    [key: string]: unknown;
}

interface TikTokComment {
    comment_id?: string;
    text?: string;
    create_time?: number;
    digg_count?: number;
    reply_count?: number;
    user?: TikTokCommentUser;
    [key: string]: unknown;
}

interface TikTokCommentsResponse {
    code?: number;
    msg?: string;
    processed_time?: number;
    data?: {
        comments?: TikTokComment[];
        hasMore?: boolean;
        cursor?: string | null;
        [key: string]: unknown;
    };
    [key: string]: unknown;
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<TikTokCommentsInput>();
        if (!input?.url) {
            throw new Error('TikTok video URL is required');
        }
        requireTikTokVideoUrl(input.url);

        const params = buildTikTokCommentsParams(input);
        console.log(`Fetching TikTok comments for: ${input.url}`);

        const client = new ScrappaClient({ apiKey });
        const response = await client.get<TikTokCommentsResponse>('/tiktok/comments/list', params);

        if (response.code !== undefined && response.code !== 0) {
            throw new Error(`Scrappa TikTok Comments API returned code ${response.code}: ${response.msg ?? 'Unknown error'}`);
        }

        const comments = response.data?.comments ?? [];

        if (comments.length > 0) {
            await Actor.pushData(comments.map((comment) => ({
                ...comment,
                video_url: input.url,
            })));
            console.log(`Found ${comments.length} comments`);
        } else {
            console.log('No comments found for the given TikTok video URL');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        const summary = {
            comments_extracted: comments.length,
            has_next_page: response.data?.hasMore ?? false,
            next_cursor: response.data?.cursor ?? null,
            processed_time: response.processed_time ?? null,
        };

        console.log('TikTok comments extraction completed successfully');
        console.log('Results summary:', JSON.stringify(summary));
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main();

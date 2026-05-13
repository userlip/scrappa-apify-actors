import { Actor } from 'apify';
import { ScrappaClient } from './shared/scrappa-client.js';
import {
    buildTikTokCommentRepliesParams,
    buildTikTokCommentsParams,
    extractTikTokVideoId,
    formatTikTokVideoUrlForLog,
    requireTikTokVideoUrl,
    resolveMaxRepliesPerComment,
    shouldIncludeTikTokCommentReplies,
} from './request-params.js';
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

interface TikTokCommentRepliesResponse {
    code?: number;
    msg?: string;
    processed_time?: number;
    data?: {
        replies?: TikTokComment[];
        hasMore?: boolean;
        cursor?: string | null;
        [key: string]: unknown;
    };
    [key: string]: unknown;
}

interface TikTokCommentDatasetItem extends TikTokComment {
    comment_type: 'comment' | 'reply';
    video_url: string;
    video_id: string;
    parent_comment_id: string | null;
    parent_comment_text: string | null;
}

interface ReplyFetchRecord {
    parent_comment_id: string;
    response: TikTokCommentRepliesResponse;
}

function assertSuccessfulResponse(
    response: TikTokCommentsResponse | TikTokCommentRepliesResponse,
    label: string,
): void {
    if (response.code !== undefined && response.code !== 0) {
        throw new Error(`Scrappa ${label} API returned code ${response.code}: ${response.msg ?? 'Unknown error'}`);
    }
}

function toCommentDatasetItem(comment: TikTokComment, input: TikTokCommentsInput, videoId: string): TikTokCommentDatasetItem {
    return {
        ...comment,
        comment_type: 'comment',
        video_url: input.url,
        video_id: videoId,
        parent_comment_id: null,
        parent_comment_text: null,
    };
}

function toReplyDatasetItem(
    reply: TikTokComment,
    parentComment: TikTokComment,
    input: TikTokCommentsInput,
    videoId: string,
): TikTokCommentDatasetItem {
    return {
        ...reply,
        comment_type: 'reply',
        video_url: input.url,
        video_id: videoId,
        parent_comment_id: parentComment.comment_id ?? null,
        parent_comment_text: parentComment.text ?? null,
    };
}

async function fetchRepliesForComment(
    client: ScrappaClient,
    comment: TikTokComment,
    input: TikTokCommentsInput,
    videoId: string,
    maxRepliesPerComment: number,
): Promise<{ rows: TikTokCommentDatasetItem[]; responses: ReplyFetchRecord[] }> {
    if (!comment.comment_id || (comment.reply_count ?? 0) < 1) {
        return { rows: [], responses: [] };
    }

    const rows: TikTokCommentDatasetItem[] = [];
    const responses: ReplyFetchRecord[] = [];
    let cursor: string | undefined;

    while (rows.length < maxRepliesPerComment) {
        const count = Math.min(50, maxRepliesPerComment - rows.length);
        const params = buildTikTokCommentRepliesParams({
            comment_id: comment.comment_id,
            video_id: videoId,
            count,
            cursor,
        });
        const response = await client.get<TikTokCommentRepliesResponse>('/tiktok/comments/replies', params);
        assertSuccessfulResponse(response, 'TikTok Comment Replies');

        responses.push({ parent_comment_id: comment.comment_id, response });

        const replies = response.data?.replies ?? [];
        rows.push(...replies.map((reply) => toReplyDatasetItem(reply, comment, input, videoId)));

        if (!response.data?.hasMore || !response.data.cursor || replies.length === 0) {
            break;
        }
        cursor = response.data.cursor ?? undefined;
    }

    return { rows, responses };
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
        const videoId = extractTikTokVideoId(input.url);
        console.log(`Fetching TikTok comments for: ${formatTikTokVideoUrlForLog(input.url)}`);

        const client = new ScrappaClient({ apiKey });
        const response = await client.get<TikTokCommentsResponse>('/tiktok/comments/list', params);
        assertSuccessfulResponse(response, 'TikTok Comments');

        const comments = response.data?.comments ?? [];
        const rows = comments.map((comment) => toCommentDatasetItem(comment, input, videoId));
        const replyResponses: ReplyFetchRecord[] = [];

        if (shouldIncludeTikTokCommentReplies(input)) {
            const maxRepliesPerComment = resolveMaxRepliesPerComment(input);
            console.log(`Fetching up to ${maxRepliesPerComment} replies for each top-level comment with replies`);

            for (const comment of comments) {
                const result = await fetchRepliesForComment(client, comment, input, videoId, maxRepliesPerComment);
                rows.push(...result.rows);
                replyResponses.push(...result.responses);
            }
        }

        if (rows.length > 0) {
            await Actor.pushData(rows);
            console.log(`Found ${comments.length} comments and ${rows.length - comments.length} replies`);
        } else {
            console.log('No comments found for the given TikTok video URL');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);
        if (replyResponses.length > 0) {
            await store.setValue('REPLIES_OUTPUT', replyResponses);
        }

        const summary = {
            comments_extracted: comments.length,
            replies_extracted: rows.length - comments.length,
            dataset_items: rows.length,
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

main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    process.exitCode = 1;
});

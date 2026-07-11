import { getAvailableCapacity, pushVideos } from './charging.js';
import type { ActorPort } from './charging.js';
import type { ChallengePostsRequest } from './input.js';
import { getVideoId, parsePage } from './response.js';
import type { ScrappaResponse } from './response.js';

interface ClientPort {
    get<T>(path: string, params: Record<string, unknown>): Promise<T>;
}

const MAX_PAGES_PER_CHALLENGE = 100;

export interface ChallengeSummary {
    challenge_id: string;
    status: 'succeeded' | 'failed' | 'charge-limit-reached' | 'page-limit-reached' | 'pagination-stalled';
    videos_saved: number;
    pages_fetched: number;
    next_cursor: string | null;
    error?: string;
}

export async function scrapeChallenge(
    client: ClientPort,
    actor: ActorPort,
    request: ChallengePostsRequest,
    seenIds: Set<string>,
): Promise<ChallengeSummary> {
    const seenCursors = new Set<string>();
    let cursor = request.initialCursor;
    let saved = 0;
    let pages = 0;

    try {
        while (saved < request.resultLimit && pages < MAX_PAGES_PER_CHALLENGE) {
            const capacity = getAvailableCapacity(actor, request.resultLimit - saved);
            if (capacity === 0) {
                return summary('charge-limit-reached');
            }

            const count = Math.min(request.pageSize, capacity, request.resultLimit - saved);
            const response = await client.get<ScrappaResponse>('/tiktok/challenges/posts', {
                challenge_id: request.challengeId,
                count,
                ...(request.region ? { region: request.region } : {}),
                ...(cursor !== undefined ? { cursor } : {}),
            });
            pages += 1;

            if (response.code !== undefined && response.code !== 0) {
                throw new Error(`Scrappa API code ${response.code}: ${response.msg ?? 'Unknown error'}`);
            }

            const page = parsePage(response.data);
            const pageIds = new Set<string>();
            const uniqueVideos = page.videos.filter((video) => {
                const id = getVideoId(video);
                if (!id || seenIds.has(id) || pageIds.has(id)) return false;
                pageIds.add(id);
                return true;
            }).slice(0, count);
            const rows = uniqueVideos.map((video) => ({
                ...video,
                challenge_id: request.challengeId,
                requested_region: request.region ?? null,
                scraped_at: new Date().toISOString(),
            }));
            const push = await pushVideos(actor, rows);
            for (const row of rows.slice(0, push.saved)) {
                const id = getVideoId(row);
                if (id) seenIds.add(id);
            }
            saved += push.saved;
            cursor = page.cursor ?? undefined;

            if (push.limitReached) return summary('charge-limit-reached');
            if (!page.hasMore || !page.cursor) return summary('succeeded');
            if (seenCursors.has(page.cursor)) return summary('pagination-stalled');
            seenCursors.add(page.cursor);
        }
        return summary(saved >= request.resultLimit ? 'succeeded' : 'page-limit-reached');
    } catch (error) {
        return { ...summary('failed'), error: error instanceof Error ? error.message : String(error) };
    }

    function summary(status: ChallengeSummary['status']): ChallengeSummary {
        return { challenge_id: request.challengeId, status, videos_saved: saved, pages_fetched: pages, next_cursor: cursor ?? null };
    }
}

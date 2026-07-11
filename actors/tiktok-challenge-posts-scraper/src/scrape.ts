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
    status: 'succeeded' | 'failed' | 'charge-limit-reached';
    videos_saved: number;
    pages_fetched: number;
    next_cursor: string | null;
    error?: string;
}

export async function scrapeChallenge(client: ClientPort, actor: ActorPort, request: ChallengePostsRequest): Promise<ChallengeSummary> {
    const seenIds = new Set<string>();
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
            const uniqueVideos = page.videos.filter((video) => {
                const id = getVideoId(video);
                if (!id || seenIds.has(id)) return false;
                seenIds.add(id);
                return true;
            }).slice(0, count);
            const rows = uniqueVideos.map((video) => ({
                ...video,
                challenge_id: request.challengeId,
                requested_region: request.region ?? null,
                scraped_at: new Date().toISOString(),
            }));
            const push = await pushVideos(actor, rows);
            saved += push.saved;
            cursor = page.cursor ?? undefined;

            if (push.limitReached) return summary('charge-limit-reached');
            if (!page.hasMore || !page.cursor || seenCursors.has(page.cursor)) break;
            seenCursors.add(page.cursor);
        }
        return summary('succeeded');
    } catch (error) {
        return { ...summary('failed'), error: error instanceof Error ? error.message : String(error) };
    }

    function summary(status: ChallengeSummary['status']): ChallengeSummary {
        return { challenge_id: request.challengeId, status, videos_saved: saved, pages_fetched: pages, next_cursor: cursor ?? null };
    }
}

export const MAX_CHALLENGE_IDS = 20;
export const MAX_RESULTS_PER_CHALLENGE = 500;
export const MAX_TOTAL_RESULTS = 2_000;
export const MAX_PAGE_SIZE = 50;

export interface ChallengePostsInput {
    challenge_ids?: unknown;
    challenge_id?: unknown;
    region?: unknown;
    cursor?: unknown;
    results_per_challenge?: unknown;
    page_size?: unknown;
}

export interface ChallengePostsRequest {
    challengeId: string;
    region?: string;
    initialCursor?: string;
    resultLimit: number;
    pageSize: number;
}

function normalizeId(value: unknown): string | null {
    if (typeof value === 'number' && Number.isSafeInteger(value)) {
        value = String(value);
    }
    if (typeof value !== 'string' || !/^\d{1,100}$/.test(value.trim())) {
        return null;
    }
    return value.trim();
}

function positiveInteger(value: unknown, fallback: number, maximum: number, name: string): number {
    if (value === undefined) {
        return fallback;
    }
    if (typeof value !== 'number' || !Number.isInteger(value) || value < 1 || value > maximum) {
        throw new Error(`${name} must be an integer between 1 and ${maximum}`);
    }
    return value;
}

export function parseInput(input: ChallengePostsInput): ChallengePostsRequest[] {
    const batchValues = Array.isArray(input.challenge_ids) ? input.challenge_ids : [];
    const values = batchValues.length > 0 ? batchValues : [input.challenge_id];
    const ids = [...new Set(values.map(normalizeId).filter((id): id is string => id !== null))];

    if (ids.length === 0) {
        throw new Error('At least one numeric TikTok challenge ID is required');
    }
    if (ids.length > MAX_CHALLENGE_IDS) {
        throw new Error(`A maximum of ${MAX_CHALLENGE_IDS} challenge IDs is allowed per run`);
    }

    const resultLimit = positiveInteger(input.results_per_challenge, 100, MAX_RESULTS_PER_CHALLENGE, 'results_per_challenge');
    if (ids.length * resultLimit > MAX_TOTAL_RESULTS) {
        throw new Error(`Total requested results cannot exceed ${MAX_TOTAL_RESULTS}`);
    }
    const pageSize = Math.min(positiveInteger(input.page_size, 10, MAX_PAGE_SIZE, 'page_size'), resultLimit);

    const region = typeof input.region === 'string' && input.region.trim() !== '' ? input.region.trim().toUpperCase() : undefined;
    if (region && !/^[A-Z]{2}$/.test(region)) {
        throw new Error('region must be a two-letter country code');
    }

    let initialCursor: string | undefined;
    if (typeof input.cursor === 'string' && input.cursor.trim() !== '') {
        initialCursor = input.cursor.trim();
    } else if (typeof input.cursor === 'number' && Number.isSafeInteger(input.cursor)) {
        initialCursor = String(input.cursor);
    } else if (input.cursor !== undefined && input.cursor !== null) {
        throw new Error('cursor must be a string or safe integer');
    }

    return ids.map((challengeId) => ({ challengeId, region, initialCursor, resultLimit, pageSize }));
}

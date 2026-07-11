export interface TikTokChallengeDetailsInput {
    challenge_names?: unknown;
    challenge_ids?: unknown;
    challenge_name?: unknown;
    challenge_id?: unknown;
}

export interface TikTokChallengeDetailsRequest {
    type: 'challenge_name' | 'challenge_id';
    value: string;
    params: Record<string, string>;
}

const MAX_ENTITIES = 100;

export function normalizeChallengeName(value: string): string {
    const name = value.trim().replace(/^#/, '');
    if (name === '') return '';
    if (!/^[^\s?#/=:]{1,255}$/u.test(name)) {
        throw new Error('TikTok challenge names must be 1 to 255 characters and cannot contain whitespace or URL delimiter characters');
    }
    return name;
}

function normalizeChallengeNameValue(value: unknown): string {
    if (typeof value !== 'string') throw new Error('TikTok challenge names must be strings');
    return normalizeChallengeName(value);
}

export function normalizeChallengeId(value: unknown): string {
    if (typeof value !== 'string' && !(typeof value === 'number' && Number.isSafeInteger(value))) {
        throw new Error('TikTok challenge IDs must be strings of digits or safe integers');
    }
    const id = typeof value === 'string' ? value.trim() : String(value);
    if (id === '') return '';
    if (!/^\d+$/.test(id) || id.length > 100) {
        throw new Error('TikTok challenge IDs must contain 1 to 100 digits');
    }
    return id;
}

function collectValues(value: unknown, field: string, normalize: (entry: unknown) => string, warn: (message: string) => void): string[] {
    const entries = Array.isArray(value) ? value : value === undefined || value === null ? [] : [value];
    if (!Array.isArray(value) && value !== undefined && value !== null && field.endsWith('s')) {
        warn(`${field} must be an array. Treating the supplied value as one lookup for API compatibility.`);
    }
    const values: string[] = [];
    for (const entry of entries) {
        try {
            const normalized = normalize(entry);
            if (normalized) values.push(normalized);
        } catch (error) {
            warn(`${field} entry omitted: ${error instanceof Error ? error.message : String(error)}`);
        }
    }
    return values;
}

export function buildTikTokChallengeDetailsRequests(input: TikTokChallengeDetailsInput, warn: (message: string) => void = console.warn): TikTokChallengeDetailsRequest[] {
    const names = [
        ...collectValues(input.challenge_names, 'challenge_names', normalizeChallengeNameValue, warn),
        ...collectValues(input.challenge_name, 'challenge_name', normalizeChallengeNameValue, warn),
    ];
    const ids = [
        ...collectValues(input.challenge_ids, 'challenge_ids', normalizeChallengeId, warn),
        ...collectValues(input.challenge_id, 'challenge_id', normalizeChallengeId, warn),
    ];
    const uniqueNames = names.filter((name, index) => names.findIndex((candidate) => candidate.toLowerCase() === name.toLowerCase()) === index);
    const uniqueIds = [...new Set(ids)];
    if (uniqueNames.length + uniqueIds.length === 0) throw new Error('At least one valid TikTok challenge name or challenge ID is required');
    if (uniqueNames.length + uniqueIds.length > MAX_ENTITIES) throw new Error(`A maximum of ${MAX_ENTITIES} combined TikTok challenge names and IDs is allowed per run`);
    return [
        ...uniqueNames.map((value) => ({ type: 'challenge_name' as const, value, params: { challenge_name: value } })),
        ...uniqueIds.map((value) => ({ type: 'challenge_id' as const, value, params: { challenge_id: value } })),
    ];
}

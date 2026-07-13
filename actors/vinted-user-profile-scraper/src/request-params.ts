export interface VintedUserProfileInput {
    user_id?: unknown;
    user_ids?: unknown;
    country?: unknown;
}

export interface VintedUserProfileRequest {
    userId: string;
    params: Record<string, unknown>;
    index: number;
}

export const VALID_VINTED_COUNTRIES = [
    'FR', 'DE', 'ES', 'IT', 'NL', 'BE', 'AT', 'PL', 'CZ', 'LT',
    'LU', 'SK', 'HU', 'RO', 'PT', 'SE', 'DK', 'FI', 'US',
] as const;

export const MAX_USER_IDS_PER_RUN = 100;
const DEFAULT_COUNTRY = 'FR';
const MAX_USER_ID_LENGTH = 32;
const MAX_BATCH_INPUT_LENGTH = 4000;

function cleanString(value: unknown, field: string, maxLength: number): string | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'string' && typeof value !== 'number') {
        throw new Error(`${field} must be a string or number`);
    }

    const normalized = String(value).trim();
    if (normalized === '') {
        return undefined;
    }

    if (normalized.length > maxLength) {
        throw new Error(`${field} must be ${maxLength} characters or fewer`);
    }

    return normalized;
}

function cleanCountry(value: unknown): string {
    const country = cleanString(value, 'country', 2) ?? DEFAULT_COUNTRY;
    const normalized = country.toUpperCase();

    if (!VALID_VINTED_COUNTRIES.includes(normalized as typeof VALID_VINTED_COUNTRIES[number])) {
        throw new Error(`country must be one of: ${VALID_VINTED_COUNTRIES.join(', ')}`);
    }

    return normalized;
}

function cleanUserId(value: unknown, field: string): string | undefined {
    if (typeof value === 'number') {
        if (!Number.isSafeInteger(value) || value < 0) {
            throw new Error(`${field} must be a numeric Vinted user ID or safe integer`);
        }

        return String(value);
    }

    const userId = cleanString(value, field, MAX_USER_ID_LENGTH);
    if (userId === undefined) {
        return undefined;
    }

    if (!/^\d+$/.test(userId)) {
        throw new Error(`${field} must be a numeric Vinted user ID; received "${userId}"`);
    }

    return userId;
}

function splitUserIds(value: unknown, field: string): string[] {
    if (value === undefined || value === null || value === '') {
        return [];
    }

    if (Array.isArray(value)) {
        return value.flatMap((entry, index) => splitUserIds(entry, `${field}[${index}]`));
    }

    const raw = cleanString(value, field, MAX_BATCH_INPUT_LENGTH);
    if (raw === undefined) {
        return [];
    }

    return raw.split(',').map((userId, index) => cleanUserId(userId, `${field}[${index}]`)).filter(
        (userId): userId is string => userId !== undefined,
    );
}

function getUserIds(input: VintedUserProfileInput): string[] {
    const values = [
        ...splitUserIds(input.user_id, 'user_id'),
        ...splitUserIds(input.user_ids, 'user_ids'),
    ];
    const unique = [...new Set(values)];

    if (unique.length === 0) {
        throw new Error('Provide at least one Vinted user ID using user_id or user_ids');
    }

    if (unique.length > MAX_USER_IDS_PER_RUN) {
        throw new Error(`user_ids supports at most ${MAX_USER_IDS_PER_RUN} unique IDs per run`);
    }

    return unique;
}

export function buildVintedUserProfileRequests(input: VintedUserProfileInput): VintedUserProfileRequest[] {
    const country = cleanCountry(input.country);
    const userIds = getUserIds(input);

    return userIds.map((userId, index) => ({
        userId,
        params: {
            user_id: userId,
            country,
        },
        index,
    }));
}

export function describeVintedUserProfileRequest(request: VintedUserProfileRequest): string {
    return `${request.userId} in ${String(request.params.country)}`;
}

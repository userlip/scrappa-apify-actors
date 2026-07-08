export interface VintedUserItemsInput {
    user_id?: unknown;
    user_ids?: unknown;
    country?: unknown;
    page?: unknown;
    per_page?: unknown;
    max_pages?: unknown;
    order?: unknown;
}

export interface VintedUserItemsPlan {
    baseParams: Record<string, unknown>;
    userIds: string[];
    startPage: number;
    perPage: number;
    maxPages: number;
}

const VALID_COUNTRIES = ['FR', 'DE', 'ES', 'IT', 'NL', 'BE', 'AT', 'PL', 'CZ', 'LT', 'LU', 'SK', 'HU', 'RO', 'PT', 'SE', 'DK', 'FI', 'US'] as const;
const VALID_ORDERS = ['newest_first', 'price_low_to_high', 'price_high_to_low', 'relevance'] as const;
const MAX_PAGE = 999;
const DEFAULT_PER_PAGE = 24;
const DEFAULT_MAX_PAGES = 1;
const MAX_PAGES_PER_USER = 20;
const MAX_USER_IDS_PER_RUN = 100;

function cleanString(value: unknown, field: string, maxLength: number): string | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'string' && typeof value !== 'number') {
        throw new Error(`${field} must be a string or number`);
    }

    const raw = String(value).trim();
    if (raw === '') {
        return undefined;
    }

    if (raw.length > maxLength) {
        throw new Error(`${field} must be ${maxLength} characters or fewer`);
    }

    return raw;
}

function cleanInteger(value: unknown, field: string, min: number, max: number): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    const normalized = typeof value === 'string' && value.trim() !== '' && /^\d+$/.test(value.trim())
        ? Number(value.trim())
        : value;

    if (typeof normalized !== 'number' || !Number.isInteger(normalized)) {
        throw new Error(`${field} must be an integer`);
    }

    if (normalized < min || normalized > max) {
        throw new Error(`${field} must be between ${min} and ${max}`);
    }

    return normalized;
}

function cleanCountry(value: unknown): string {
    const country = cleanString(value, 'country', 2);
    if (country === undefined) {
        return 'FR';
    }

    const normalized = country.toUpperCase();
    if (!VALID_COUNTRIES.includes(normalized as typeof VALID_COUNTRIES[number])) {
        throw new Error(`country must be one of: ${VALID_COUNTRIES.join(', ')}`);
    }

    return normalized;
}

function cleanOrder(value: unknown): string | undefined {
    const order = cleanString(value, 'order', 30);
    if (order === undefined) {
        return undefined;
    }

    if (!VALID_ORDERS.includes(order as typeof VALID_ORDERS[number])) {
        throw new Error(`order must be one of: ${VALID_ORDERS.join(', ')}`);
    }

    return order;
}

function splitUserIds(value: unknown, field: string): string[] {
    if (value === undefined || value === null || value === '') {
        return [];
    }

    if (Array.isArray(value)) {
        return value.flatMap((entry) => splitUserIds(entry, field));
    }

    const raw = cleanString(value, field, 2000);
    if (raw === undefined) {
        return [];
    }

    return raw.split(',')
        .map((userId) => userId.trim())
        .filter(Boolean);
}

function cleanUserIds(input: VintedUserItemsInput): string[] {
    const userIds = [...splitUserIds(input.user_id, 'user_id'), ...splitUserIds(input.user_ids, 'user_ids')];
    const uniqueUserIds = [...new Set(userIds)];

    if (uniqueUserIds.length === 0) {
        throw new Error('Provide at least one Vinted seller user_id or user_ids value');
    }

    if (uniqueUserIds.length > MAX_USER_IDS_PER_RUN) {
        throw new Error(`user_ids supports at most ${MAX_USER_IDS_PER_RUN} unique IDs per run`);
    }

    const invalidUserId = uniqueUserIds.find((userId) => !/^\d+$/.test(userId));
    if (invalidUserId) {
        throw new Error(`user_ids must contain numeric Vinted user IDs; received "${invalidUserId}"`);
    }

    return uniqueUserIds;
}

export function buildVintedUserItemsPlan(input: VintedUserItemsInput): VintedUserItemsPlan {
    const baseParams: Record<string, unknown> = {
        country: cleanCountry(input.country),
        per_page: cleanInteger(input.per_page, 'per_page', 1, 100) ?? DEFAULT_PER_PAGE,
    };

    const order = cleanOrder(input.order);
    if (order !== undefined) {
        baseParams.order = order;
    }

    const startPage = cleanInteger(input.page, 'page', 1, MAX_PAGE) ?? 1;
    const maxPages = cleanInteger(input.max_pages, 'max_pages', 1, MAX_PAGES_PER_USER) ?? DEFAULT_MAX_PAGES;
    if (startPage + maxPages - 1 > MAX_PAGE) {
        throw new Error('page plus max_pages cannot exceed page 999');
    }

    return {
        baseParams,
        userIds: cleanUserIds(input),
        startPage,
        perPage: Number(baseParams.per_page),
        maxPages,
    };
}

export function buildUserPageParams(plan: VintedUserItemsPlan, userId: string, page: number): Record<string, unknown> {
    return {
        ...plan.baseParams,
        user_id: userId,
        page,
    };
}

export function describeVintedUserItemsRequest(plan: VintedUserItemsPlan): string {
    const sellerDescription = plan.userIds.length === 1
        ? `seller ${plan.userIds[0]}`
        : `${plan.userIds.length} sellers`;
    const pageDescription = plan.maxPages === 1
        ? `page ${plan.startPage}`
        : `pages ${plan.startPage}-${plan.startPage + plan.maxPages - 1}`;

    return `${sellerDescription} in ${String(plan.baseParams.country)} (${pageDescription}, ${plan.perPage}/page)`;
}

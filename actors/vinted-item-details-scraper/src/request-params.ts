export interface VintedItemDetailsInput {
    item_id?: unknown;
    item_ids?: unknown;
    country?: unknown;
}

export interface VintedItemDetailsRequest {
    itemId: string;
    params: Record<string, unknown>;
    index: number;
}

const VALID_COUNTRIES = ['FR', 'DE', 'ES', 'IT', 'NL', 'BE', 'AT', 'PL', 'CZ', 'LT', 'LU', 'SK', 'HU', 'RO', 'PT', 'SE', 'DK', 'FI', 'US'] as const;
const MAX_ITEM_IDS_PER_RUN = 50;

function cleanString(value: unknown, field: string, maxLength: number): string | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    const normalized = typeof value === 'number' && Number.isInteger(value) ? String(value) : value;
    if (typeof normalized !== 'string') {
        throw new Error(`${field} must be a string or integer`);
    }

    const trimmed = normalized.trim();
    if (trimmed === '') {
        return undefined;
    }

    if (trimmed.length > maxLength) {
        throw new Error(`${field} must be ${maxLength} characters or fewer`);
    }

    return trimmed;
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

function cleanItemId(value: unknown, field: string): string | undefined {
    const itemId = cleanString(value, field, 32);
    if (itemId === undefined) {
        return undefined;
    }

    if (!/^\d+$/.test(itemId)) {
        throw new Error(`${field} must be a numeric Vinted item ID`);
    }

    return itemId;
}

function getInputItemIds(input: VintedItemDetailsInput): string[] {
    const itemIds: string[] = [];
    const seen = new Set<string>();

    const addItemId = (value: unknown, field: string): void => {
        const itemId = cleanItemId(value, field);
        if (itemId === undefined || seen.has(itemId)) {
            return;
        }

        seen.add(itemId);
        itemIds.push(itemId);
    };

    addItemId(input.item_id, 'item_id');

    if (Array.isArray(input.item_ids)) {
        input.item_ids.forEach((itemId, index) => addItemId(itemId, `item_ids[${index}]`));
    } else if (input.item_ids !== undefined) {
        throw new Error('item_ids must be an array of numeric Vinted item IDs');
    }

    return itemIds;
}

export function buildVintedItemDetailsRequests(input: VintedItemDetailsInput): VintedItemDetailsRequest[] {
    const country = cleanCountry(input.country);
    const itemIds = getInputItemIds(input);

    if (itemIds.length === 0) {
        throw new Error('Provide at least one Vinted item ID using item_id or item_ids');
    }

    if (itemIds.length > MAX_ITEM_IDS_PER_RUN) {
        throw new Error(`item_ids cannot include more than ${MAX_ITEM_IDS_PER_RUN} IDs per run`);
    }

    return itemIds.map((itemId, index) => ({
        itemId,
        params: {
            item_id: itemId,
            country,
        },
        index,
    }));
}

export function describeVintedItemDetailsRequest(request: VintedItemDetailsRequest): string {
    return `${request.itemId} in ${String(request.params.country)}`;
}

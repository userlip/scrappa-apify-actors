export interface BookingSearchResponse {
    data?: {
        results?: BookingPropertyResult[];
        [key: string]: unknown;
    };
    results?: BookingPropertyResult[];
    [key: string]: unknown;
}

export interface BookingPropertyResult {
    name?: string;
    title?: string;
    url?: string;
    link?: string;
    image?: string;
    thumbnail?: string;
    review_score?: number | string;
    review_score_word?: string;
    review_count?: number | string;
    location?: string;
    address?: string;
    price?: string;
    price_for_display?: string;
    currency?: string;
    [key: string]: unknown;
}

function firstString(...values: unknown[]): string | null {
    for (const value of values) {
        if (typeof value === 'string' && value.trim() !== '') {
            return value;
        }
    }

    return null;
}

function firstNumber(...values: unknown[]): number | null {
    for (const value of values) {
        if (typeof value === 'number' && Number.isFinite(value)) {
            return value;
        }
        if (typeof value === 'string' && value.trim() !== '') {
            const number = Number(value);
            if (Number.isFinite(number)) {
                return number;
            }
        }
    }

    return null;
}

export function getBookingSearchResults(response: BookingSearchResponse): BookingPropertyResult[] {
    if (Array.isArray(response.data?.results)) {
        return response.data.results;
    }

    if (Array.isArray(response.results)) {
        return response.results;
    }

    return [];
}

export function buildBookingDatasetItem(
    property: BookingPropertyResult,
    params: Record<string, unknown>,
    searchIndex: number,
): Record<string, unknown> {
    return {
        ...property,
        name: firstString(property.name, property.title),
        url: firstString(property.url, property.link),
        image: firstString(property.image, property.thumbnail),
        review_score: firstNumber(property.review_score),
        review_score_word: firstString(property.review_score_word),
        review_count: firstNumber(property.review_count),
        location: firstString(property.location, property.address),
        price: firstString(property.price, property.price_for_display),
        currency: firstString(property.currency, params.currency),
        request_search_index: searchIndex,
        request_ss: params.ss ?? null,
        request_checkin: params.checkin ?? null,
        request_checkout: params.checkout ?? null,
        request_group_adults: params.group_adults ?? null,
        request_group_children: params.group_children ?? null,
        request_no_rooms: params.no_rooms ?? null,
        request_lang: params.lang ?? null,
        request_currency: params.currency ?? null,
    };
}

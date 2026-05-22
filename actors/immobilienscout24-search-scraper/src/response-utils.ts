export interface Immobilienscout24Listing {
    id?: string | null;
    online_id?: string | null;
    title?: string | null;
    price?: number | null;
    price_formatted?: string | null;
    rooms?: number | null;
    rooms_max?: number | null;
    size_m2?: number | null;
    size_m2_max?: number | null;
    address?: string | null;
    lat?: number | null;
    lon?: number | null;
    image_url?: string | null;
    url?: string | null;
    is_private?: boolean | null;
    published?: string | null;
    [key: string]: unknown;
}

export interface Immobilienscout24SearchResponse {
    success?: boolean;
    total_results?: number;
    page?: number;
    total_pages?: number;
    results?: Immobilienscout24Listing[];
    data?: {
        results?: Immobilienscout24Listing[];
        total_results?: number;
        page?: number;
        total_pages?: number;
        [key: string]: unknown;
    };
    [key: string]: unknown;
}

export function getImmobilienscout24Listings(
    response: Immobilienscout24SearchResponse | null | undefined,
): Immobilienscout24Listing[] {
    const topLevelResults = response?.results;
    const wrappedResults = response?.data?.results;

    if (Array.isArray(topLevelResults) && (topLevelResults.length > 0 || !Array.isArray(wrappedResults))) {
        return topLevelResults;
    }

    if (Array.isArray(wrappedResults)) {
        return wrappedResults;
    }

    console.debug('Unexpected ImmobilienScout24 response shape: expected "results" or "data.results" array.');
    return [];
}

export function getImmobilienscout24TotalResults(response: Immobilienscout24SearchResponse | null | undefined): number | null {
    return getNumber(response?.total_results) ?? getNumber(response?.data?.total_results) ?? null;
}

export function getImmobilienscout24Page(response: Immobilienscout24SearchResponse | null | undefined): number | null {
    return getNumber(response?.page) ?? getNumber(response?.data?.page) ?? null;
}

export function getImmobilienscout24TotalPages(response: Immobilienscout24SearchResponse | null | undefined): number | null {
    return getNumber(response?.total_pages) ?? getNumber(response?.data?.total_pages) ?? null;
}

export function buildImmobilienscout24DatasetItem(
    listing: Immobilienscout24Listing,
    params: Record<string, unknown>,
): Record<string, unknown> {
    return {
        ...listing,
        id: listing.id ?? null,
        online_id: listing.online_id ?? null,
        title: listing.title ?? null,
        price: listing.price ?? null,
        price_formatted: listing.price_formatted ?? null,
        rooms: listing.rooms ?? null,
        rooms_max: listing.rooms_max ?? null,
        size_m2: listing.size_m2 ?? null,
        size_m2_max: listing.size_m2_max ?? null,
        address: listing.address ?? null,
        latitude: listing.lat ?? null,
        longitude: listing.lon ?? null,
        image_url: listing.image_url ?? null,
        url: listing.url ?? null,
        is_private: listing.is_private ?? null,
        published: listing.published ?? null,
        request_location: params.location ?? null,
        request_type: params.type ?? null,
        request_price_min: params.price_min ?? null,
        request_price_max: params.price_max ?? null,
        request_rooms_min: params.rooms_min ?? null,
        request_rooms_max: params.rooms_max ?? null,
        request_size_min: params.size_min ?? null,
        request_size_max: params.size_max ?? null,
        request_page: params.page ?? null,
        request_per_page: params.per_page ?? null,
    };
}

export function limitImmobilienscout24SearchResponse(
    response: Immobilienscout24SearchResponse,
    limit: number,
): Immobilienscout24SearchResponse {
    const limitedResponse: Immobilienscout24SearchResponse = { ...response };

    if (Array.isArray(response.results)) {
        limitedResponse.results = response.results.slice(0, limit);
    }

    if (Array.isArray(response.data?.results)) {
        limitedResponse.data = {
            ...response.data,
            results: response.data.results.slice(0, limit),
        };
    }

    return limitedResponse;
}

function getNumber(value: unknown): number | undefined {
    if (typeof value === 'number' && Number.isFinite(value)) {
        return value;
    }

    if (typeof value === 'string' && /^\d+$/.test(value.trim())) {
        return Number(value.trim());
    }

    return undefined;
}

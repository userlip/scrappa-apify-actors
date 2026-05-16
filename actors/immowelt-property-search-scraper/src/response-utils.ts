export interface ImmoweltPropertyListing {
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

export interface ImmoweltPropertySearchResponse {
    success?: boolean;
    total_results?: number;
    page?: number;
    total_pages?: number;
    results?: ImmoweltPropertyListing[];
    data?: {
        results?: ImmoweltPropertyListing[];
        total_results?: number;
        page?: number;
        total_pages?: number;
        [key: string]: unknown;
    };
    [key: string]: unknown;
}

export function getImmoweltPropertyListings(
    response: ImmoweltPropertySearchResponse,
): ImmoweltPropertyListing[] {
    if (Array.isArray(response.results)) {
        return response.results;
    }

    if (Array.isArray(response.data?.results)) {
        return response.data.results;
    }

    console.debug('Unexpected Immowelt response shape: expected "results" or "data.results" array.');
    return [];
}

export function getImmoweltTotalResults(response: ImmoweltPropertySearchResponse): number | null {
    return getNumber(response.total_results) ?? getNumber(response.data?.total_results) ?? null;
}

export function getImmoweltPage(response: ImmoweltPropertySearchResponse): number | null {
    return getNumber(response.page) ?? getNumber(response.data?.page) ?? null;
}

export function getImmoweltTotalPages(response: ImmoweltPropertySearchResponse): number | null {
    return getNumber(response.total_pages) ?? getNumber(response.data?.total_pages) ?? null;
}

export function buildImmoweltDatasetItem(
    listing: ImmoweltPropertyListing,
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
        request_property_type: params.property_type ?? null,
        request_page: params.page ?? null,
        request_limit: params.limit ?? null,
    };
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

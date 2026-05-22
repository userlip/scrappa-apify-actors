export interface KleinanzeigenListing {
    id?: string | number | null;
    title?: string | null;
    url?: string | null;
    price?: string | number | null;
    price_numeric?: number | null;
    location?: string | null;
    image?: string | null;
    image_url?: string | null;
    description?: string | null;
    has_shipping?: boolean | null;
    [key: string]: unknown;
}

export interface KleinanzeigenSearchResponse {
    success?: boolean;
    data?: KleinanzeigenListing[] | {
        listings?: KleinanzeigenListing[];
        results?: KleinanzeigenListing[];
        items?: KleinanzeigenListing[];
        [key: string]: unknown;
    };
    listings?: KleinanzeigenListing[];
    results?: KleinanzeigenListing[];
    items?: KleinanzeigenListing[];
    meta?: {
        query?: string;
        page?: number;
        location?: string | null;
        category?: string | null;
        results_count?: number;
        duration_ms?: number;
        [key: string]: unknown;
    };
    [key: string]: unknown;
}

export function getKleinanzeigenListings(
    response: KleinanzeigenSearchResponse | null | undefined,
): KleinanzeigenListing[] {
    if (Array.isArray(response?.data)) {
        return response.data;
    }

    if (Array.isArray(response?.listings)) {
        return response.listings;
    }

    if (Array.isArray(response?.results)) {
        return response.results;
    }

    if (Array.isArray(response?.items)) {
        return response.items;
    }

    if (response?.data && typeof response.data === 'object') {
        if (Array.isArray(response.data.listings)) {
            return response.data.listings;
        }

        if (Array.isArray(response.data.results)) {
            return response.data.results;
        }

        if (Array.isArray(response.data.items)) {
            return response.data.items;
        }
    }

    console.debug('Unexpected Kleinanzeigen response shape: expected "data", "listings", "results", or "items" array.');
    return [];
}

export function buildKleinanzeigenDatasetItem(
    listing: KleinanzeigenListing,
    params: Record<string, unknown>,
    response: KleinanzeigenSearchResponse,
): Record<string, unknown> {
    return {
        ...listing,
        id: listing.id ?? null,
        title: listing.title ?? null,
        url: listing.url ?? null,
        price: listing.price ?? null,
        price_numeric: listing.price_numeric ?? null,
        location: listing.location ?? null,
        image_url: listing.image_url ?? listing.image ?? null,
        description: listing.description ?? null,
        has_shipping: listing.has_shipping ?? null,
        request_query: params.query ?? null,
        request_page: params.page ?? null,
        request_location: params.location ?? null,
        request_category: params.category ?? null,
        request_price_min: params.price_min ?? null,
        request_price_max: params.price_max ?? null,
        results_count: response.meta?.results_count ?? null,
    };
}

export function limitKleinanzeigenSearchResponse(
    response: KleinanzeigenSearchResponse,
    limit: number,
): KleinanzeigenSearchResponse {
    const limitedResponse: KleinanzeigenSearchResponse = { ...response };

    if (Array.isArray(response.data)) {
        limitedResponse.data = response.data.slice(0, limit);
    } else if (response.data && typeof response.data === 'object') {
        limitedResponse.data = { ...response.data };

        if (Array.isArray(response.data.listings)) {
            limitedResponse.data.listings = response.data.listings.slice(0, limit);
        }

        if (Array.isArray(response.data.results)) {
            limitedResponse.data.results = response.data.results.slice(0, limit);
        }

        if (Array.isArray(response.data.items)) {
            limitedResponse.data.items = response.data.items.slice(0, limit);
        }
    }

    if (Array.isArray(response.listings)) {
        limitedResponse.listings = response.listings.slice(0, limit);
    }

    if (Array.isArray(response.results)) {
        limitedResponse.results = response.results.slice(0, limit);
    }

    if (Array.isArray(response.items)) {
        limitedResponse.items = response.items.slice(0, limit);
    }

    return limitedResponse;
}

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

export type KleinanzeigenListingsSource =
    | 'data'
    | 'listings'
    | 'results'
    | 'items'
    | 'data.listings'
    | 'data.results'
    | 'data.items';

export interface KleinanzeigenListingsSelection {
    listings: KleinanzeigenListing[];
    source: KleinanzeigenListingsSource | null;
}

export function getKleinanzeigenListings(
    response: KleinanzeigenSearchResponse | null | undefined,
): KleinanzeigenListing[] {
    return selectKleinanzeigenListings(response).listings;
}

export function selectKleinanzeigenListings(
    response: KleinanzeigenSearchResponse | null | undefined,
): KleinanzeigenListingsSelection {
    const candidates: Array<{
        source: KleinanzeigenListingsSource;
        listings: KleinanzeigenListing[];
    }> = [];

    const data = response?.data;

    if (Array.isArray(data)) {
        candidates.push({ source: 'data', listings: data });
    }

    if (Array.isArray(response?.listings)) {
        candidates.push({ source: 'listings', listings: response.listings });
    }

    if (Array.isArray(response?.results)) {
        candidates.push({ source: 'results', listings: response.results });
    }

    if (Array.isArray(response?.items)) {
        candidates.push({ source: 'items', listings: response.items });
    }

    if (data && !Array.isArray(data) && typeof data === 'object') {
        if (Array.isArray(data.listings)) {
            candidates.push({ source: 'data.listings', listings: data.listings });
        }

        if (Array.isArray(data.results)) {
            candidates.push({ source: 'data.results', listings: data.results });
        }

        if (Array.isArray(data.items)) {
            candidates.push({ source: 'data.items', listings: data.items });
        }
    }

    const populatedCandidate = candidates.find((candidate) => candidate.listings.length > 0);
    if (populatedCandidate) {
        return populatedCandidate;
    }

    if (candidates.length > 0) {
        return {
            source: candidates[0].source,
            listings: [],
        };
    }

    console.warn('Unexpected Kleinanzeigen response shape: expected "data", "listings", "results", or "items" array.');
    return {
        source: null,
        listings: [],
    };
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
    response: KleinanzeigenSearchResponse | null | undefined,
    limit: number,
    selectedSource?: KleinanzeigenListingsSource | null,
): KleinanzeigenSearchResponse {
    if (!response) {
        return {};
    }

    const source = selectedSource === undefined
        ? selectKleinanzeigenListings(response).source
        : selectedSource;
    const limitedResponse: KleinanzeigenSearchResponse = { ...response };

    if (Array.isArray(response.data) && source === 'data') {
        limitedResponse.data = response.data.slice(0, limit);
    } else if (Array.isArray(response.data)) {
        delete limitedResponse.data;
    } else if (response.data && typeof response.data === 'object') {
        limitedResponse.data = { ...response.data };

        if (Array.isArray(response.data.listings) && source === 'data.listings') {
            limitedResponse.data.listings = response.data.listings.slice(0, limit);
        } else {
            delete limitedResponse.data.listings;
        }

        if (Array.isArray(response.data.results) && source === 'data.results') {
            limitedResponse.data.results = response.data.results.slice(0, limit);
        } else {
            delete limitedResponse.data.results;
        }

        if (Array.isArray(response.data.items) && source === 'data.items') {
            limitedResponse.data.items = response.data.items.slice(0, limit);
        } else {
            delete limitedResponse.data.items;
        }
    }

    if (Array.isArray(response.listings) && source === 'listings') {
        limitedResponse.listings = response.listings.slice(0, limit);
    } else {
        delete limitedResponse.listings;
    }

    if (Array.isArray(response.results) && source === 'results') {
        limitedResponse.results = response.results.slice(0, limit);
    } else {
        delete limitedResponse.results;
    }

    if (Array.isArray(response.items) && source === 'items') {
        limitedResponse.items = response.items.slice(0, limit);
    } else {
        delete limitedResponse.items;
    }

    return limitedResponse;
}

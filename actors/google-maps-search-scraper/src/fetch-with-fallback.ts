export interface BusinessResult {
    name?: string;
    // Scrappa documents Google Maps price_level as a string value.
    price_level?: string | null;
    price_level_text?: string | null;
    review_count?: number | null;
    rating?: number | null;
    website?: string | null;
    domain?: string | null;
    latitude?: number | null;
    longitude?: number | null;
    business_id?: string | null;
    subtypes?: string[];
    district?: string | null;
    full_address?: string | null;
    timezone?: string | null;
    short_description?: string | null;
    full_description?: string | null;
    owner_id?: string | null;
    owner_name?: string | null;
    owner_link?: string | null;
    order_link?: string | null;
    google_mid?: string | null;
    type?: string | null;
    phone_numbers?: string[];
    place_id?: string | null;
    photos_sample?: BusinessPhotoSample[];
    opening_hours?: BusinessOpeningHours[];
    current_status?: string | null;
    [key: string]: unknown;
}

export interface BusinessPhotoSample {
    photo_id?: string;
    photo_url?: string;
    photo_url_large?: string;
    video_thumbnail_url?: string | null;
    latitude?: number | null;
    longitude?: number | null;
    type?: string | null;
    [key: string]: unknown;
}

export interface BusinessOpeningHours {
    day?: string;
    hours?: string;
    date?: string;
    special_day?: boolean;
    [key: string]: unknown;
}

export interface GoogleMapsSearchResponse {
    items?: BusinessResult[];
    [key: string]: unknown;
}

export interface ScrappaLikeClient {
    get<T>(endpoint: string, params?: Record<string, unknown>): Promise<T>;
}

export function isTransientUpstreamError(message: string): boolean {
    if (/Scrappa API error \((408|429|5\d\d)\)/.test(message)) {
        return true;
    }

    return /timed?\s*out|timeout|temporarily unavailable|cloudflare/i.test(message);
}

export async function fetchWithFallback(
    client: ScrappaLikeClient,
    baseParams: Record<string, unknown>,
    fallbackZoom: number
): Promise<GoogleMapsSearchResponse> {
    try {
        return await client.get<GoogleMapsSearchResponse>('/maps/simple-search', baseParams);
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        if (!isTransientUpstreamError(message)) {
            throw error;
        }

        console.warn(`Transient upstream issue on simple-search: ${message}`);
    }

    const advancedParams: Record<string, unknown> = {
        ...baseParams,
        zoom: fallbackZoom,
    };

    const advancedResponse = await client.get<GoogleMapsSearchResponse>('/maps/advanced-search', advancedParams);
    return {
        ...advancedResponse,
        fallback_used: 'advanced-search',
    };
}

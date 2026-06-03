export interface BusinessIdNormalization {
    businessId: string;
    source: 'business_id' | 'place_id' | 'url';
}

export interface GoogleMapsPhotosInput {
    business_id?: string;
    business_ids?: string[];
    use_cache?: boolean;
    maximum_cache_age?: number;
}

export interface BusinessIdRequest {
    input_business_id: string;
    business_id?: string;
    source?: BusinessIdNormalization['source'];
    validation_error?: string;
}

export const MAX_BUSINESS_IDS_PER_RUN = 10;

export function getBusinessIdRequests(input: GoogleMapsPhotosInput | null): BusinessIdRequest[] {
    const rawBusinessIds = [
        ...(typeof input?.business_id === 'string' ? [input.business_id] : []),
        ...(Array.isArray(input?.business_ids) ? input.business_ids : []),
    ];

    const seen = new Set<string>();
    const requests: BusinessIdRequest[] = [];

    for (const rawBusinessId of rawBusinessIds) {
        const inputBusinessId = rawBusinessId.trim();
        if (!inputBusinessId) {
            continue;
        }

        try {
            const normalized = normalizeBusinessId(inputBusinessId);
            if (seen.has(normalized.businessId)) {
                continue;
            }

            seen.add(normalized.businessId);
            requests.push({
                input_business_id: inputBusinessId,
                business_id: normalized.businessId,
                source: normalized.source,
            });
        } catch (error) {
            const key = `invalid:${inputBusinessId}`;
            if (seen.has(key)) {
                continue;
            }

            seen.add(key);
            requests.push({
                input_business_id: inputBusinessId,
                validation_error: error instanceof Error ? error.message : String(error),
            });
        }
    }

    if (requests.length > MAX_BUSINESS_IDS_PER_RUN) {
        throw new Error(`business_ids must contain ${MAX_BUSINESS_IDS_PER_RUN} unique items or fewer`);
    }

    return requests;
}

export function normalizeBusinessId(rawValue: string): BusinessIdNormalization {
    const value = rawValue.trim();
    if (!value) {
        throw new Error('Business ID is required');
    }

    const decodedValue = decodeRepeatedly(value);
    const googleId = findGoogleBusinessId(decodedValue);
    if (googleId) {
        return {
            businessId: googleId,
            source: value === googleId ? 'business_id' : 'url',
        };
    }

    const placeId = findPlaceId(decodedValue);
    if (placeId) {
        return {
            businessId: placeId,
            source: value === placeId ? 'place_id' : 'url',
        };
    }

    if (looksLikeGoogleMapsUrl(decodedValue)) {
        throw new Error(
            'Google Maps URL must contain an extractable 0x...:0x... business ID or ChIJ... place ID. Use a business_id from a Google Maps Search or Business Details actor run.'
        );
    }

    return {
        businessId: value,
        source: 'business_id',
    };
}

function decodeRepeatedly(value: string): string {
    let decoded = value;

    for (let i = 0; i < 3; i += 1) {
        try {
            const next = decodeURIComponent(decoded);
            if (next === decoded) {
                return next;
            }
            decoded = next;
        } catch {
            return decoded;
        }
    }

    return decoded;
}

function findGoogleBusinessId(value: string): string | null {
    return value.match(/0x[0-9a-f]+:0x[0-9a-f]+/i)?.[0] ?? null;
}

function findPlaceId(value: string): string | null {
    return value.match(/ChIJ[A-Za-z0-9_-]+/)?.[0] ?? null;
}

function looksLikeGoogleMapsUrl(value: string): boolean {
    return /^https?:\/\/(?:www\.|maps\.)?google\.(?:[a-z]{2,3})(?:\.[a-z]{2})?\/maps(?:[/?#]|$)/i.test(value)
        || /^https?:\/\/maps\.app\.goo\.gl\//i.test(value);
}

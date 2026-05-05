export interface BusinessIdNormalization {
    businessId: string;
    source: 'business_id' | 'place_id' | 'url';
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
    return /^https?:\/\/(?:www\.)?google\.(?:com|[a-z]{2}(?:\.[a-z]{2})?)\/maps\//i.test(value)
        || /^https?:\/\/maps\.app\.goo\.gl\//i.test(value);
}

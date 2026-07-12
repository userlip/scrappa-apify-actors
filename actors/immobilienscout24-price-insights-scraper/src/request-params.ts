export interface PriceInsightsInput {
    locations?: unknown;
    location?: unknown;
}

export interface PriceInsightsRequest {
    location: string;
    index: number;
}

const MAX_LOCATIONS_PER_RUN = 100;
const MAX_LOCATION_LENGTH = 120;

export function normalizeLocations(input: PriceInsightsInput | null | undefined): PriceInsightsRequest[] {
    if (!input) {
        throw new Error('Input is required');
    }

    const values = getLocationValues(input);
    const seen = new Set<string>();
    const locations: string[] = [];

    for (const [index, value] of values.entries()) {
        if (typeof value !== 'string') {
            throw new Error(`locations[${index}] must be a string`);
        }

        const location = value.trim();
        if (!location) {
            continue;
        }
        if (location.length > MAX_LOCATION_LENGTH) {
            throw new Error(`locations[${index}] must be ${MAX_LOCATION_LENGTH} characters or fewer`);
        }

        const deduplicationKey = location.toLocaleLowerCase('de-DE');
        if (!seen.has(deduplicationKey)) {
            seen.add(deduplicationKey);
            locations.push(location);
        }
    }

    if (locations.length === 0) {
        throw new Error('Provide at least one non-empty location in locations or location');
    }
    if (locations.length > MAX_LOCATIONS_PER_RUN) {
        throw new Error(`A run can include at most ${MAX_LOCATIONS_PER_RUN} unique locations`);
    }

    return locations.map((location, index) => ({ location, index }));
}

function getLocationValues(input: PriceInsightsInput): unknown[] {
    if (input.locations !== undefined) {
        if (Array.isArray(input.locations)) {
            return input.locations;
        }
        if (typeof input.locations === 'string') {
            return input.locations.split(',');
        }
        throw new Error('locations must be an array of strings or a comma-separated string');
    }

    return input.location === undefined ? [] : [input.location];
}

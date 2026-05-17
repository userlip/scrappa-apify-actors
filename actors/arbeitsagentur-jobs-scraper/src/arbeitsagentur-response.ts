export interface ArbeitsagenturLocation {
    ort?: string | null;
    plz?: string | null;
    region?: string | null;
    land?: string | null;
    koordinaten?: Record<string, unknown> | null;
    entfernung?: string | number | null;
    [key: string]: unknown;
}

export interface ArbeitsagenturJob {
    refnr?: string | null;
    titel?: string | null;
    beruf?: string | null;
    arbeitgeber?: string | null;
    arbeitsort?: ArbeitsagenturLocation | string | null;
    aktuelleVeroeffentlichungsdatum?: string | null;
    eintrittsdatum?: string | null;
    externeUrl?: string | null;
    [key: string]: unknown;
}

export interface ArbeitsagenturDatasetJob extends ArbeitsagenturJob {
    title: string | null;
    occupation: string | null;
    company_name: string | null;
    location_formatted: string | null;
    location_city: string | null;
    postal_code: string | null;
    region: string | null;
    country: string | null;
    published_date: string | null;
    start_date: string | null;
    job_url: string | null;
    reference_number: string | null;
    distance_km: string | number | null;
    latitude: unknown;
    longitude: unknown;
}

export interface ArbeitsagenturJobsData {
    stellenangebote?: ArbeitsagenturJob[];
    maxErgebnisse?: number | string;
    page?: number;
    size?: number;
    facetten?: unknown;
    [key: string]: unknown;
}

export interface ArbeitsagenturJobsResponse {
    success?: boolean;
    data?: ArbeitsagenturJobsData;
    stellenangebote?: ArbeitsagenturJob[];
    maxErgebnisse?: number | string;
    page?: number;
    size?: number;
    facetten?: unknown;
    [key: string]: unknown;
}

export function getArbeitsagenturJobs(response: ArbeitsagenturJobsResponse): ArbeitsagenturJob[] {
    if (Array.isArray(response.data?.stellenangebote)) {
        return response.data.stellenangebote;
    }

    if (Array.isArray(response.stellenangebote)) {
        return response.stellenangebote;
    }

    console.debug('Unexpected Arbeitsagentur Jobs response shape: expected "data.stellenangebote" or "stellenangebote" array.');
    return [];
}

export function getArbeitsagenturMetadata(response: ArbeitsagenturJobsResponse): ArbeitsagenturJobsData | undefined {
    return response.data ?? response;
}

export function toArbeitsagenturDatasetJob(job: ArbeitsagenturJob): ArbeitsagenturDatasetJob {
    const location = getLocationParts(job.arbeitsort);
    const coordinates = location.koordinaten && typeof location.koordinaten === 'object' ? location.koordinaten : {};

    return {
        ...job,
        title: job.titel ?? null,
        occupation: job.beruf ?? null,
        company_name: job.arbeitgeber ?? null,
        location_formatted: getFormattedLocation(job.arbeitsort) ?? null,
        location_city: location.ort ?? null,
        postal_code: location.plz ?? null,
        region: location.region ?? null,
        country: location.land ?? null,
        published_date: job.aktuelleVeroeffentlichungsdatum ?? null,
        start_date: job.eintrittsdatum ?? null,
        job_url: job.externeUrl ?? null,
        reference_number: job.refnr ?? null,
        distance_km: location.entfernung ?? null,
        latitude: coordinates.lat ?? null,
        longitude: coordinates.lon ?? null,
    };
}

export function getFormattedLocation(location: ArbeitsagenturJob['arbeitsort']): string | undefined {
    if (typeof location === 'string') {
        return location;
    }

    if (!location || typeof location !== 'object') {
        return undefined;
    }

    const parts = [location.plz, location.ort, location.region, location.land]
        .filter((part): part is string => typeof part === 'string' && part.trim() !== '');

    return parts.length > 0 ? parts.join(', ') : undefined;
}

function getLocationParts(location: ArbeitsagenturJob['arbeitsort']): ArbeitsagenturLocation {
    if (!location || typeof location !== 'object') {
        return {};
    }

    return location;
}

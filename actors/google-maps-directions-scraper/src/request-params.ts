export interface DirectionsInput {
    routes?: unknown;
    origin?: unknown;
    destination?: unknown;
    mode?: unknown;
    hl?: unknown;
    gl?: unknown;
}

export interface DirectionsRouteInput {
    origin?: unknown;
    destination?: unknown;
    mode?: unknown;
    hl?: unknown;
    gl?: unknown;
}

export interface DirectionsRequest {
    origin: string;
    destination: string;
    mode: DirectionsMode;
    hl: string;
    gl?: string;
    params: Record<string, string>;
    index: number;
}

export type DirectionsMode = 'driving' | 'walking' | 'bicycling' | 'transit';

export const MAX_ROUTES_PER_RUN = 10;

const MAX_LOCATION_LENGTH = 200;
const MAX_LANGUAGE_LENGTH = 5;
const MAX_REGION_LENGTH = 2;
const VALID_MODES = new Set<DirectionsMode>(['driving', 'walking', 'bicycling', 'transit']);

function cleanRequiredString(value: unknown, field: string, maxLength: number): string {
    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const trimmed = value.trim();
    if (!trimmed) {
        throw new Error(`${field} must not be empty`);
    }
    if (trimmed.length > maxLength) {
        throw new Error(`${field} must be ${maxLength} characters or fewer`);
    }

    return trimmed;
}

function cleanOptionalString(value: unknown, field: string, maxLength: number): string | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const trimmed = value.trim();
    if (!trimmed) {
        return undefined;
    }
    if (trimmed.length > maxLength) {
        throw new Error(`${field} must be ${maxLength} characters or fewer`);
    }

    return trimmed;
}

function cleanMode(value: unknown, field: string): DirectionsMode {
    const mode = cleanOptionalString(value, field, 10)?.toLowerCase() ?? 'driving';
    const normalized = mode === 'cycling' ? 'bicycling' : mode;
    if (!VALID_MODES.has(normalized as DirectionsMode)) {
        throw new Error(`${field} must be one of driving, walking, bicycling, cycling, or transit`);
    }

    return normalized as DirectionsMode;
}

function cleanLanguage(value: unknown, field: string): string {
    const language = cleanOptionalString(value, field, MAX_LANGUAGE_LENGTH)?.toLowerCase() ?? 'en';
    if (!/^[a-z]{2}(?:-[a-z]{2})?$/.test(language)) {
        throw new Error(`${field} must be a language code such as en or de-DE`);
    }

    return language;
}

function cleanRegion(value: unknown, field: string): string | undefined {
    const region = cleanOptionalString(value, field, MAX_REGION_LENGTH)?.toLowerCase();
    if (region !== undefined && !/^[a-z]{2}$/.test(region)) {
        throw new Error(`${field} must be a two-letter country or region code`);
    }

    return region;
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function getRouteInputs(input: DirectionsInput): DirectionsRouteInput[] {
    const routeInputs: DirectionsRouteInput[] = [];
    const hasSingular = input.origin !== undefined || input.destination !== undefined;

    if (hasSingular) {
        if (input.origin === undefined || input.destination === undefined) {
            throw new Error('origin and destination must be provided together');
        }
        routeInputs.push({
            origin: input.origin,
            destination: input.destination,
            mode: input.mode,
            hl: input.hl,
            gl: input.gl,
        });
    }

    if (input.routes !== undefined) {
        if (!Array.isArray(input.routes)) {
            throw new Error('routes must be an array of route objects');
        }

        input.routes.forEach((route, index) => {
            if (!isRecord(route)) {
                throw new Error(`routes[${index}] must be an object`);
            }
            routeInputs.push(route);
        });
    }

    return routeInputs;
}

function normalizeRoute(route: DirectionsRouteInput, index: number): DirectionsRequest {
    const origin = cleanRequiredString(route.origin, `routes[${index}].origin`, MAX_LOCATION_LENGTH);
    const destination = cleanRequiredString(route.destination, `routes[${index}].destination`, MAX_LOCATION_LENGTH);
    const mode = cleanMode(route.mode, `routes[${index}].mode`);
    const hl = cleanLanguage(route.hl, `routes[${index}].hl`);
    const gl = cleanRegion(route.gl, `routes[${index}].gl`);
    const params: Record<string, string> = { origin, destination, mode, hl };
    if (gl !== undefined) {
        params.gl = gl;
    }

    return { origin, destination, mode, hl, gl, params, index };
}

function deduplicationKey(request: DirectionsRequest): string {
    return [request.origin, request.destination, request.mode, request.hl, request.gl ?? '']
        .map((value) => value.trim().toLocaleLowerCase('en-US'))
        .join('\u0000');
}

export function buildDirectionsRequests(input: DirectionsInput | null | undefined): DirectionsRequest[] {
    if (!input) {
        throw new Error('Input is required');
    }

    const routeInputs = getRouteInputs(input);
    if (routeInputs.length === 0) {
        throw new Error('Provide at least one route in routes or origin and destination');
    }

    const seen = new Set<string>();
    const requests: DirectionsRequest[] = [];
    routeInputs.forEach((route, sourceIndex) => {
        const request = normalizeRoute(route, sourceIndex);
        const key = deduplicationKey(request);
        if (seen.has(key)) {
            return;
        }
        seen.add(key);
        requests.push({ ...request, index: requests.length });
    });

    if (requests.length > MAX_ROUTES_PER_RUN) {
        throw new Error(`A run can include at most ${MAX_ROUTES_PER_RUN} unique routes`);
    }

    return requests;
}

export function describeDirectionsRequest(request: DirectionsRequest): string {
    return `${request.origin} -> ${request.destination} (${request.mode})`;
}

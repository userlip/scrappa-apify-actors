export interface GoogleVideosInput {
    q?: unknown;
    page?: unknown;
    start?: unknown;
    hl?: unknown;
    gl?: unknown;
    google_domain?: unknown;
    location?: unknown;
    uule?: unknown;
    tbs?: unknown;
    safe?: unknown;
    filter?: unknown;
    nfpr?: unknown;
    lr?: unknown;
}

const SAFE_VALUES = ['active', 'off'] as const;

function cleanString(value: unknown, field: string, maxLength: number): string | undefined {
    if (value === undefined || value === null) {
        return undefined;
    }

    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const trimmed = value.trim();
    if (trimmed === '') {
        return undefined;
    }

    if (trimmed.length > maxLength) {
        throw new Error(`${field} must be ${maxLength} characters or fewer`);
    }

    return trimmed;
}

function cleanRequiredString(value: unknown, field: string, maxLength: number): string {
    const cleaned = cleanString(value, field, maxLength);
    if (cleaned === undefined) {
        throw new Error(`${field} is required`);
    }

    return cleaned;
}

function cleanTwoLetterCode(value: unknown, field: string): string | undefined {
    const cleaned = cleanString(value, field, 2);
    if (cleaned === undefined) {
        return undefined;
    }

    if (!/^[a-z]{2}$/i.test(cleaned)) {
        throw new Error(`${field} must be a two-letter code`);
    }

    return cleaned.toLowerCase();
}

function cleanInteger(value: unknown, field: string, min: number): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'number' || !Number.isInteger(value)) {
        throw new Error(`${field} must be an integer`);
    }

    if (value < min) {
        throw new Error(`${field} must be greater than or equal to ${min}`);
    }

    return value;
}

function cleanZeroOne(value: unknown, field: string): 0 | 1 | undefined {
    const cleaned = cleanInteger(value, field, 0);
    if (cleaned === undefined) {
        return undefined;
    }

    if (cleaned !== 0 && cleaned !== 1) {
        throw new Error(`${field} must be 0 or 1`);
    }

    return cleaned;
}

function cleanEnum<T extends readonly string[]>(
    value: unknown,
    field: string,
    allowedValues: T,
): T[number] | undefined {
    const cleaned = cleanString(value, field, 100);
    if (cleaned === undefined) {
        return undefined;
    }

    const normalized = cleaned.toLowerCase();
    if (!allowedValues.includes(normalized)) {
        throw new Error(`${field} must be one of: ${allowedValues.join(', ')}`);
    }

    return normalized;
}

export function buildGoogleVideosParams(input: GoogleVideosInput): Record<string, unknown> {
    const params: Record<string, unknown> = {
        q: cleanRequiredString(input.q, 'q', 500),
    };

    const page = cleanInteger(input.page, 'page', 1);
    const start = cleanInteger(input.start, 'start', 0);
    if (page !== undefined && start !== undefined) {
        throw new Error('Cannot use both page and start parameters');
    }

    const location = cleanString(input.location, 'location', 500);
    const uule = cleanString(input.uule, 'uule', 500);
    if (location !== undefined && uule !== undefined) {
        throw new Error('Cannot use both location and uule parameters');
    }

    const hl = cleanTwoLetterCode(input.hl, 'hl');
    const gl = cleanTwoLetterCode(input.gl, 'gl');
    const googleDomain = cleanString(input.google_domain, 'google_domain', 50);
    const tbs = cleanString(input.tbs, 'tbs', 500);
    const safe = cleanEnum(input.safe, 'safe', SAFE_VALUES);
    const filter = cleanZeroOne(input.filter, 'filter');
    const nfpr = cleanZeroOne(input.nfpr, 'nfpr');
    const lr = cleanString(input.lr, 'lr', 200);

    if (page !== undefined) params.page = page;
    if (start !== undefined) params.start = start;
    if (hl !== undefined) params.hl = hl;
    if (gl !== undefined) params.gl = gl;
    if (googleDomain !== undefined) params.google_domain = googleDomain;
    if (location !== undefined) params.location = location;
    if (uule !== undefined) params.uule = uule;
    if (tbs !== undefined) params.tbs = tbs;
    if (safe !== undefined) params.safe = safe;
    if (filter !== undefined) params.filter = filter;
    if (nfpr !== undefined) params.nfpr = nfpr;
    if (lr !== undefined) params.lr = lr;

    return params;
}

export function describeGoogleVideosRequest(params: Record<string, unknown>): string {
    const paginationParts = [];
    if (typeof params.page === 'number') {
        paginationParts.push(`page ${params.page}`);
    }
    if (typeof params.start === 'number') {
        paginationParts.push(`start ${params.start}`);
    }

    const filters = ['google_domain', 'hl', 'gl', 'location', 'uule', 'tbs', 'safe', 'filter', 'nfpr', 'lr']
        .filter((field) => params[field] !== undefined)
        .map((field) => `${field}=${String(params[field])}`);
    const suffixParts = [...paginationParts, ...filters];
    const suffix = suffixParts.length > 0 ? ` (${suffixParts.join(', ')})` : '';
    const query = typeof params.q === 'string' ? params.q : 'unknown query';

    return `query "${query}"${suffix}`;
}

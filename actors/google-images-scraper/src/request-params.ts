export interface GoogleImagesInput {
    q?: unknown;
    queries?: unknown;
    page?: unknown;
    hl?: unknown;
    gl?: unknown;
    imgsz?: unknown;
    imgtype?: unknown;
    imgcolor?: unknown;
    imgar?: unknown;
    tbs?: unknown;
    safe?: unknown;
}

const MAX_QUERIES_PER_RUN = 50;

const IMAGE_SIZE_VALUES = ['large', 'medium', 'icon'] as const;
const IMAGE_TYPE_VALUES = ['photo', 'clipart', 'lineart', 'gif', 'face'] as const;
const IMAGE_COLOR_VALUES = [
    'color',
    'gray',
    'trans',
    'red',
    'orange',
    'yellow',
    'green',
    'teal',
    'blue',
    'purple',
    'pink',
    'white',
    'black',
    'brown',
] as const;
const ASPECT_RATIO_VALUES = ['tall', 'square', 'wide'] as const;
const SAFE_VALUES = ['active', 'off'] as const;

function cleanString(value: unknown, field: string, maxLength: number): string | undefined {
    if (value === undefined || value === null || value === '') {
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

export function buildGoogleImagesParams(input: GoogleImagesInput): Record<string, unknown> {
    const params: Record<string, unknown> = {
        q: cleanRequiredString(input.q, 'q', 500),
    };

    const page = cleanInteger(input.page, 'page', 1);
    const hl = cleanTwoLetterCode(input.hl, 'hl');
    const gl = cleanTwoLetterCode(input.gl, 'gl');
    const imgsz = cleanEnum(input.imgsz, 'imgsz', IMAGE_SIZE_VALUES);
    const imgtype = cleanEnum(input.imgtype, 'imgtype', IMAGE_TYPE_VALUES);
    const imgcolor = cleanEnum(input.imgcolor, 'imgcolor', IMAGE_COLOR_VALUES);
    const imgar = cleanEnum(input.imgar, 'imgar', ASPECT_RATIO_VALUES);
    const tbs = cleanString(input.tbs, 'tbs', 500);
    const safe = cleanEnum(input.safe, 'safe', SAFE_VALUES);

    if (page !== undefined) params.page = page;
    if (hl !== undefined) params.hl = hl;
    if (gl !== undefined) params.gl = gl;
    if (imgsz !== undefined) params.imgsz = imgsz;
    if (imgtype !== undefined) params.imgtype = imgtype;
    if (imgcolor !== undefined) params.imgcolor = imgcolor;
    if (imgar !== undefined) params.imgar = imgar;
    if (tbs !== undefined) params.tbs = tbs;
    if (safe !== undefined) params.safe = safe;

    return params;
}

export function buildGoogleImagesParamList(input: GoogleImagesInput): Record<string, unknown>[] {
    const queries = getGoogleImagesQueries(input);
    return queries.map((query) => buildGoogleImagesParams({
        ...input,
        q: query,
        queries: undefined,
    }));
}

function getGoogleImagesQueries(input: GoogleImagesInput): string[] {
    const rawQueries = [
        ...(input.q !== undefined && input.q !== null && input.q !== '' ? [input.q] : []),
    ];

    if (input.queries !== undefined && input.queries !== null && input.queries !== '') {
        if (!Array.isArray(input.queries)) {
            throw new Error('queries must be an array');
        }

        if (input.queries.length > MAX_QUERIES_PER_RUN) {
            throw new Error(`queries must contain ${MAX_QUERIES_PER_RUN} items or fewer`);
        }

        rawQueries.push(...input.queries);
    }

    const seen = new Set<string>();
    const queries: string[] = [];
    for (const rawQuery of rawQueries) {
        const query = cleanString(rawQuery, 'q', 500);
        if (query === undefined || seen.has(query)) {
            continue;
        }

        seen.add(query);
        queries.push(query);
    }

    if (queries.length === 0) {
        throw new Error('q is required');
    }

    if (queries.length > MAX_QUERIES_PER_RUN) {
        throw new Error(`queries must contain ${MAX_QUERIES_PER_RUN} items or fewer`);
    }

    return queries;
}

export function describeGoogleImagesRequest(params: Record<string, unknown>): string {
    const filters = ['imgsz', 'imgtype', 'imgcolor', 'imgar', 'tbs', 'safe']
        .filter((field) => params[field] !== undefined)
        .map((field) => `${field}=${String(params[field])}`);
    const page = typeof params.page === 'number' ? ` page ${params.page}` : '';
    const filterSuffix = filters.length > 0 ? ` (${filters.join(', ')})` : '';

    const query = typeof params.q === 'string' ? params.q : 'unknown query';

    return `query "${query}"${page}${filterSuffix}`;
}

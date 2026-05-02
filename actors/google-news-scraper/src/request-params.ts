export interface GoogleNewsInput {
    q?: unknown;
    gl?: unknown;
    hl?: unknown;
    page?: unknown;
    start?: unknown;
    so?: unknown;
    topic_token?: unknown;
    kgmid?: unknown;
    publication_token?: unknown;
    section_token?: unknown;
    story_token?: unknown;
}

const TOKEN_FIELDS = ['topic_token', 'kgmid', 'publication_token', 'section_token', 'story_token'] as const;

type TokenField = typeof TOKEN_FIELDS[number];

function cleanOptionalString(value: unknown, field: string, maxLength: number): string | undefined {
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

function cleanTwoLetterCode(value: unknown, field: string): string | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const cleaned = value.trim();
    if (cleaned === '') {
        return undefined;
    }

    if (!/^[a-z]{2}$/i.test(cleaned)) {
        throw new Error(`${field} must be a two-letter code`);
    }

    return cleaned.toLowerCase();
}

function cleanInteger(value: unknown, field: string, min: number, max?: number): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'number' || !Number.isInteger(value)) {
        throw new Error(`${field} must be an integer`);
    }

    if (value < min || (max !== undefined && value > max)) {
        throw new Error(max === undefined
            ? `${field} must be greater than or equal to ${min}`
            : `${field} must be between ${min} and ${max}`);
    }

    return value;
}

export function buildGoogleNewsParams(input: GoogleNewsInput): Record<string, unknown> {
    const q = cleanOptionalString(input.q, 'q', 500);
    const tokenParams = new Map<TokenField, string>();

    for (const field of TOKEN_FIELDS) {
        const maxLength = field === 'story_token' ? 2000 : field === 'kgmid' ? 100 : 200;
        const value = cleanOptionalString(input[field], field, maxLength);
        if (value !== undefined) {
            tokenParams.set(field, value);
        }
    }

    if (!q && tokenParams.size === 0) {
        throw new Error('Provide q or one Google News token parameter');
    }

    if (q && tokenParams.size > 0) {
        throw new Error('Cannot use q with topic_token, kgmid, publication_token, section_token, or story_token');
    }

    if (tokenParams.has('kgmid') && tokenParams.size > 1) {
        throw new Error('kgmid must be used alone and cannot be combined with other token parameters');
    }

    const kgmid = tokenParams.get('kgmid');
    if (kgmid !== undefined && !/^\/[mg]\//.test(kgmid)) {
        throw new Error('kgmid must start with /m/ or /g/');
    }

    const page = cleanInteger(input.page, 'page', 1);
    const start = cleanInteger(input.start, 'start', 0);
    if (page !== undefined && start !== undefined) {
        throw new Error('Cannot use both page and start parameters');
    }

    const so = cleanInteger(input.so, 'so', 0, 1);

    const params: Record<string, unknown> = {};
    if (q !== undefined) params.q = q;
    if (input.gl !== undefined) params.gl = cleanTwoLetterCode(input.gl, 'gl');
    if (input.hl !== undefined) params.hl = cleanTwoLetterCode(input.hl, 'hl');
    if (page !== undefined) params.page = page;
    if (start !== undefined) params.start = start;
    if (so !== undefined) params.so = so;

    for (const [field, value] of tokenParams.entries()) {
        params[field] = value;
    }

    return params;
}

export function describeGoogleNewsRequest(params: Record<string, unknown>): string {
    if (typeof params.q === 'string') {
        return `query "${params.q}"`;
    }

    const token = TOKEN_FIELDS.find((field) => typeof params[field] === 'string');
    return token ? `${token} ${String(params[token])}` : 'Google News request';
}

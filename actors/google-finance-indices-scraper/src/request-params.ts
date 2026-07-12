export interface GoogleFinanceIndicesInput {
    indices?: unknown;
    hl?: unknown;
    gl?: unknown;
}

export interface IndicesParams {
    indices?: string;
    hl: string;
    gl: string;
}

export const MAX_INDICES = 50;

export function normalizeIndices(value: unknown): string[] {
    let values: unknown[];
    if (Array.isArray(value)) {
        values = value;
    } else if (typeof value === 'string') {
        values = value.split(',');
    } else if (value === null || value === undefined) {
        values = [];
    } else {
        throw new Error('indices must be a comma-separated string or an array of strings');
    }

    const result: string[] = [];

    for (const candidate of values) {
        if (typeof candidate !== 'string') {
            throw new Error('indices entries must be strings');
        }

        const symbol = candidate.trim().toUpperCase();

        if (!symbol) {
            continue;
        }

        if (symbol.length > 64 || !/^[A-Z0-9._:-]+$/.test(symbol)) {
            throw new Error(`Invalid index symbol: ${candidate.trim() || '(empty)'}`);
        }

        if (!result.includes(symbol)) {
            result.push(symbol);
        }
    }

    if (result.length > MAX_INDICES) {
        throw new Error(`A maximum of ${MAX_INDICES} indices is allowed per run`);
    }

    return result;
}

function locale(value: unknown, field: 'hl' | 'gl', pattern: RegExp, fallback: string): string {
    const normalized = value === null || value === undefined
        ? fallback
        : typeof value === 'string'
            ? value.trim().toLowerCase()
            : '';

    if (!pattern.test(normalized)) {
        const label = field === 'gl' ? 'two-letter country' : 'language';
        throw new Error(`${field} must be a valid ${label} code`);
    }

    return normalized;
}

export function buildGoogleFinanceIndicesParams(input: GoogleFinanceIndicesInput): IndicesParams {
    const indices = normalizeIndices(input.indices);
    const params: IndicesParams = {
        hl: locale(input.hl, 'hl', /^[a-z]{2,3}(?:-[a-z]{2,4})?$/, 'en'),
        gl: locale(input.gl, 'gl', /^[a-z]{2}$/, 'us'),
    };

    if (indices.length > 0) {
        params.indices = indices.join(',');
    }

    return params;
}

export function describeGoogleFinanceIndicesRequest(params: IndicesParams): string {
    const symbols = params.indices ?? 'default indices';
    return `${symbols} (hl=${params.hl}, gl=${params.gl})`;
}

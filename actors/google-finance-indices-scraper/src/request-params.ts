export interface GoogleFinanceIndicesInput { indices?: unknown; hl?: unknown; gl?: unknown; }
export interface IndicesParams { indices?: string; hl: string; gl: string; }
export const MAX_INDICES = 50;

export function normalizeIndices(value: unknown): string[] {
    const values = Array.isArray(value) ? value : typeof value === 'string' ? value.split(',') : value == null ? [] : (() => { throw new Error('indices must be a comma-separated string or an array of strings'); })();
    const result: string[] = [];
    for (const candidate of values) {
        if (typeof candidate !== 'string') throw new Error('indices entries must be strings');
        const symbol = candidate.trim().toUpperCase();
        if (!symbol) continue;
        if (symbol.length > 64 || !/^[A-Z0-9._:-]+$/.test(symbol)) throw new Error(`Invalid index symbol: ${candidate.trim() || '(empty)'}`);
        if (!result.includes(symbol)) result.push(symbol);
    }
    if (result.length > MAX_INDICES) throw new Error(`A maximum of ${MAX_INDICES} indices is allowed per run`);
    return result;
}
function locale(value: unknown, field: 'hl' | 'gl', pattern: RegExp, fallback: string): string {
    const normalized = value == null ? fallback : typeof value === 'string' ? value.trim().toLowerCase() : '';
    if (!pattern.test(normalized)) throw new Error(`${field} must be a valid ${field === 'gl' ? 'two-letter country' : 'language'} code`);
    return normalized;
}
export function buildGoogleFinanceIndicesParams(input: GoogleFinanceIndicesInput): IndicesParams {
    const indices = normalizeIndices(input.indices);
    return { ...(indices.length ? { indices: indices.join(',') } : {}), hl: locale(input.hl, 'hl', /^[a-z]{2,3}(?:-[a-z]{2,4})?$/, 'en'), gl: locale(input.gl, 'gl', /^[a-z]{2}$/, 'us') };
}
export function describeGoogleFinanceIndicesRequest(params: IndicesParams): string { return `${params.indices ?? 'default indices'} (hl=${params.hl}, gl=${params.gl})`; }

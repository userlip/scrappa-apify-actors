export interface GoogleFinanceIndexRow { symbol?: unknown; name?: unknown; exchange?: unknown; price?: unknown; current_price?: unknown; price_change?: unknown; change?: unknown; percent_change?: unknown; price_change_percent?: unknown; previous_close?: unknown; movement_direction?: unknown; price_movement_direction?: unknown; [key: string]: unknown; }
export interface GoogleFinanceIndicesResponse { data?: unknown; indices?: unknown; results?: unknown; [key: string]: unknown; }
export interface IndexItem { id: string; requested_symbol: string; symbol: string; name: string | null; exchange: string | null; current_price: number | null; price_change: number | null; percent_change: number | null; previous_close: number | null; movement_direction: string | null; request_hl: string; request_gl: string; retrieved_at: string; }
const string = (value: unknown): string | null => typeof value === 'string' && value.trim() ? value.trim() : null;
const number = (value: unknown): number | null => typeof value === 'number' && Number.isFinite(value) ? value : typeof value === 'string' && value.trim() && Number.isFinite(Number(value.replace(/[%,$]/g, ''))) ? Number(value.replace(/[%,$]/g, '')) : null;
export function canonicalSymbol(value: unknown): string | null { const symbol = string(value); return symbol ? symbol.toUpperCase() : null; }
export function extractIndexRows(response: GoogleFinanceIndicesResponse): GoogleFinanceIndexRow[] {
    const candidate = response.data ?? response.indices ?? response.results;
    if (Array.isArray(candidate)) return candidate.filter((item): item is GoogleFinanceIndexRow => !!item && typeof item === 'object');
    if (candidate && typeof candidate === 'object') { const nested = (candidate as Record<string, unknown>).indices ?? (candidate as Record<string, unknown>).results; if (Array.isArray(nested)) return nested.filter((item): item is GoogleFinanceIndexRow => !!item && typeof item === 'object'); }
    return [];
}
export function mapIndexRow(row: GoogleFinanceIndexRow, requestedSymbol: string, params: { hl: string; gl: string }, retrievedAt = new Date().toISOString()): IndexItem | null {
    const symbol = canonicalSymbol(row.symbol); if (!symbol) return null;
    const exchange = string(row.exchange); const id = `${exchange?.toUpperCase() ?? 'UNKNOWN'}:${symbol}`;
    return { id, requested_symbol: requestedSymbol, symbol, name: string(row.name), exchange, current_price: number(row.current_price ?? row.price), price_change: number(row.price_change ?? row.change), percent_change: number(row.percent_change ?? row.price_change_percent), previous_close: number(row.previous_close), movement_direction: string(row.movement_direction ?? row.price_movement_direction), request_hl: params.hl, request_gl: params.gl, retrieved_at: retrievedAt };
}

import { canonicalSymbol, extractIndexRows, mapIndexRow } from './response-utils.js';
import type { GoogleFinanceIndicesResponse, IndexItem } from './response-utils.js';

export const INDEX_RESULT_CHARGE_EVENT = 'index-result';

export interface BatchDependencies {
    getCapacity(): number;
    fetch(symbol?: string): Promise<GoogleFinanceIndicesResponse>;
    save(item: IndexItem): Promise<{ savedCount: number; chargeLimitReached: boolean }>;
}

export interface BatchSummary {
    requested: number;
    attempted: number;
    saved: number;
    duplicate: number;
    failed: number;
    charged: number;
    charge_limit_reached: boolean;
    outcomes: Array<{ symbol: string; status: string; error?: string }>;
}

export async function runIndicesBatch(
    requested: string[],
    params: { hl: string; gl: string },
    dependencies: BatchDependencies,
): Promise<BatchSummary> {
    const outcomes: BatchSummary['outcomes'] = [];
    let saved = 0;
    let duplicate = 0;
    let failed = 0;
    let chargeLimitReached = false;
    const seen = new Set<string>();

    if (dependencies.getCapacity() <= 0) {
        return {
            requested: requested.length,
            attempted: 0,
            saved,
            duplicate,
            failed,
            charged: saved,
            charge_limit_reached: true,
            outcomes: requested.map((symbol) => ({
                symbol,
                status: 'not_attempted',
                error: 'Charge limit reached',
            })),
        };
    }

    // The upstream endpoint currently accepts a symbol but returns only one
    // row for a comma-separated list. Fetch each requested symbol in the same
    // Actor run. Requests run concurrently, so the longest single 90-second
    // retry budget remains below the 20-second shutdown reserve.
    const symbolsToFetch = requested.length > 0 ? requested : [undefined];
    const capacity = dependencies.getCapacity();
    const fetchCount = Number.isFinite(capacity)
        ? Math.min(symbolsToFetch.length, Math.max(0, Math.floor(capacity)))
        : symbolsToFetch.length;
    const fetchedSymbols = symbolsToFetch.slice(0, fetchCount);
    const fetchLimited = fetchedSymbols.length < symbolsToFetch.length;

    // An exhausted Scrappa request is an Actor-level failure. Keep this
    // rejection intact so main() invokes Actor.fail() instead of publishing a
    // misleading successful zero-result summary.
    const responses = await Promise.all(fetchedSymbols.map(async (symbol) => {
        try {
            return { symbol, response: await dependencies.fetch(symbol) };
        } catch (error) {
            return { symbol, error: error instanceof Error ? error : new Error(String(error)) };
        }
    }));

    // Retain the Actor-level failure for a wholly exhausted upstream batch,
    // while allowing other requested symbols to be saved after one fails.
    if (responses.length > 0 && responses.every((response) => 'error' in response)) {
        throw responses[0].error ?? new Error('Scrappa request failed');
    }

    for (const responseResult of responses) {
        const { symbol: requestedSymbol } = responseResult;
        if ('error' in responseResult) {
            const error = responseResult.error ?? new Error('Scrappa request failed');
            failed += 1;
            outcomes.push({
                symbol: requestedSymbol ?? 'default',
                status: 'failed',
                error: error.message,
            });
            continue;
        }

        const { response } = responseResult;
        const rows = extractIndexRows(response);

        for (const row of rows) {
            const sourceSymbol = canonicalSymbol(row.symbol);

            if (!sourceSymbol || (requestedSymbol !== undefined && sourceSymbol !== requestedSymbol)) {
                failed += 1;
                outcomes.push({
                    symbol: sourceSymbol ?? 'unknown',
                    status: 'failed',
                    error: 'Scrappa returned an index outside the requested symbol',
                });
                continue;
            }

            const item = mapIndexRow(row, sourceSymbol, params);
            if (!item) {
                failed += 1;
                outcomes.push({
                    symbol: sourceSymbol,
                    status: 'failed',
                    error: 'Scrappa returned an index without a canonical symbol',
                });
                continue;
            }

            if (seen.has(item.id)) {
                duplicate += 1;
                outcomes.push({
                    symbol: sourceSymbol,
                    status: 'duplicate',
                    error: `Duplicate index ${item.id}; result was not saved or charged`,
                });
                continue;
            }

            if (dependencies.getCapacity() <= 0) {
                chargeLimitReached = true;
                outcomes.push({
                    symbol: sourceSymbol,
                    status: 'not_attempted',
                    error: 'Charge limit reached',
                });
                break;
            }

            const result = await dependencies.save(item);
            if (result.savedCount === 1) {
                seen.add(item.id);
                saved += 1;
                outcomes.push({ symbol: sourceSymbol, status: 'saved' });
            } else {
                failed += 1;
                outcomes.push({
                    symbol: sourceSymbol,
                    status: 'failed',
                    error: 'Apify did not save a chargeable index result',
                });
            }

            if (result.chargeLimitReached) {
                chargeLimitReached = true;
                break;
            }
        }

        if (chargeLimitReached) {
            break;
        }
    }

    for (const symbol of requested) {
        const hasOutcome = outcomes.some((outcome) => outcome.symbol === symbol);

        if (!hasOutcome) {
            outcomes.push({
                symbol,
                status: (chargeLimitReached || fetchLimited) ? 'not_attempted' : 'failed',
                error: (chargeLimitReached || fetchLimited)
                    ? 'Charge limit reached'
                    : 'Scrappa returned no matching index result',
            });
            if (!chargeLimitReached && !fetchLimited) {
                failed += 1;
            }
        }
    }

    return {
        requested: requested.length,
        attempted: responses.length,
        saved,
        duplicate,
        failed,
        charged: saved,
        charge_limit_reached: chargeLimitReached || fetchLimited,
        outcomes,
    };
}

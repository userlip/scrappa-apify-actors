import { canonicalSymbol, extractIndexRows, mapIndexRow } from './response-utils.js';
import type { GoogleFinanceIndicesResponse, IndexItem } from './response-utils.js';

export const INDEX_RESULT_CHARGE_EVENT = 'index-result';

export interface BatchDependencies {
    getCapacity(): number;
    fetch(): Promise<GoogleFinanceIndicesResponse>;
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
    let limited = false;
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

    // An exhausted Scrappa request is an Actor-level failure.  Keep this
    // rejection intact so main() invokes Actor.fail() instead of publishing a
    // misleading successful zero-result summary.
    const response = await dependencies.fetch();

    const wanted = new Set(requested);
    const filterRequestedSymbols = wanted.size > 0;
    const rows = extractIndexRows(response);

    for (const row of rows) {
        const sourceSymbol = canonicalSymbol(row.symbol);

        if (!sourceSymbol || (filterRequestedSymbols && !wanted.has(sourceSymbol))) {
            failed += 1;
            outcomes.push({
                symbol: sourceSymbol ?? 'unknown',
                status: 'failed',
                error: 'Scrappa returned an index outside the requested batch',
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
            limited = true;
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
            limited = true;
            break;
        }
    }

    for (const symbol of requested) {
        const hasOutcome = outcomes.some((outcome) => outcome.symbol === symbol);

        if (!hasOutcome) {
            failed += 1;
            outcomes.push({
                symbol,
                status: 'failed',
                error: 'Scrappa returned no matching index result',
            });
        }
    }

    return {
        requested: requested.length,
        attempted: 1,
        saved,
        duplicate,
        failed,
        charged: saved,
        charge_limit_reached: limited,
        outcomes,
    };
}

import { Actor } from 'apify';
import { INDEX_RESULT_CHARGE_EVENT, runIndicesBatch } from './batch-runner.js';
import { saveIndex } from './charged-save.js';
import { buildGoogleFinanceIndicesParams, describeGoogleFinanceIndicesRequest, normalizeIndices } from './request-params.js';
import type { GoogleFinanceIndicesInput } from './request-params.js';
import type { GoogleFinanceIndicesResponse } from './response-utils.js';
import {
    REQUEST_TIMEOUT_MS,
    retryBudgetFitsActorTimeout,
} from './runtime-config.js';
import { ScrappaClient } from './shared/scrappa-client.js';

async function main(): Promise<void> {
    await Actor.init();

    try {
        if (!retryBudgetFitsActorTimeout()) {
            throw new Error('Configured retry budget exceeds the Actor timeout');
        }

        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<GoogleFinanceIndicesInput>() ?? {};
        const requested = normalizeIndices(input.indices);
        const params = buildGoogleFinanceIndicesParams(input);

        console.log(`Fetching Google Finance indices for ${describeGoogleFinanceIndicesRequest(params)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: REQUEST_TIMEOUT_MS });
        const manager = Actor.getChargingManager();
        const summary = await runIndicesBatch(requested, params, {
            getCapacity: () => manager.getPricingInfo().isPayPerEvent
                ? manager.calculateMaxEventChargeCountWithinLimit(INDEX_RESULT_CHARGE_EVENT)
                : Infinity,
            fetch: (symbol) => client.get<GoogleFinanceIndicesResponse>('/google-finance/indices', {
                ...params,
                ...(symbol === undefined ? {} : { indices: symbol }),
            }),
            save: (item) => saveIndex(item, manager, Actor, INDEX_RESULT_CHARGE_EVENT),
        });

        console.log('Google Finance indices completed:', JSON.stringify(summary));
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error(`Actor failed: ${message}`);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`Actor failed: ${message}`);
    process.exitCode = 1;
});

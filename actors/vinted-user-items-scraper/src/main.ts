import { Actor } from 'apify';
import {
    buildVintedUserItemsPlan,
    describeVintedUserItemsRequest,
} from './request-params.js';
import type { VintedUserItemsInput } from './request-params.js';
import { runVintedUserItems } from './run-user-items.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const USER_ITEM_RESULT_CHARGE_EVENT = 'user-item-result';

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<VintedUserItemsInput>() ?? {};
        const plan = buildVintedUserItemsPlan(input);
        console.log(`Fetching Vinted user items for ${describeVintedUserItemsRequest(plan)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
        const summary = await runVintedUserItems({
            client,
            dataset: {
                // Apify types do not expose pay-per-event charge metadata, but runtime returns it.
                pushData: (items, eventName) => eventName === undefined
                    ? Actor.pushData(items)
                    : Actor.pushData(items, eventName),
            },
            plan,
            isPayPerEvent,
            chargeEventName: USER_ITEM_RESULT_CHARGE_EVENT,
            attempts: SCRAPPA_MAX_ATTEMPTS,
        });

        console.log('Vinted user items completed successfully');
        console.log('Results summary:', JSON.stringify({
            users_requested: plan.userIds.length,
            pages_fetched: summary.pagesFetched,
            items_extracted: summary.savedItems,
            status_message: summary.statusMessage,
        }));

        if (summary.statusMessage) {
            await Actor.exit({ statusMessage: summary.statusMessage });
            return;
        }
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Vinted user-items request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer users, fewer pages, or run the request again.`
            : rawMessage;
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    process.exitCode = 1;
});

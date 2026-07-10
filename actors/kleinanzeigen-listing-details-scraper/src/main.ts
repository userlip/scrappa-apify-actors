import { Actor } from 'apify';
import { buildKleinanzeigenDetailsPlan, describeKleinanzeigenDetailsRequest } from './request-params.js';
import type { KleinanzeigenDetailsInput } from './request-params.js';
import { buildListingDetailsOutput, processKleinanzeigenListingDetails } from './listing-processing.js';
import type { KleinanzeigenDetailsResponse } from './response-utils.js';
import { errorSummary, ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;
async function main(): Promise<void> {
    await Actor.init();
    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        const plan = buildKleinanzeigenDetailsPlan(await Actor.getInput<KleinanzeigenDetailsInput>() ?? {});
        console.log(`Fetching ${describeKleinanzeigenDetailsRequest(plan)}`);
        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const result = await processKleinanzeigenListingDetails(Actor, plan.listings, (adId) => (
            client.get<KleinanzeigenDetailsResponse>(
                '/kleinanzeigen/details',
                { ad_id: adId },
                { attempts: SCRAPPA_MAX_ATTEMPTS },
            )
        ));
        await (await Actor.openKeyValueStore()).setValue('OUTPUT', buildListingDetailsOutput(plan.listings.length, result));
        if (result.statusMessage) { await Actor.exit({ statusMessage: result.statusMessage }); return; }
    } catch (error) {
        const rawMessage = errorSummary(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Kleinanzeigen details request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout.`
            : rawMessage;
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }
    await Actor.exit();
}

main().catch((error) => { console.error('Actor failed: ' + errorSummary(error)); process.exitCode = 1; });

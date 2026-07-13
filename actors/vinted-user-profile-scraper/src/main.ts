import { Actor } from 'apify';
import {
    buildVintedUserProfileRequests,
    describeVintedUserProfileRequest,
} from './request-params.js';
import type { VintedUserProfileInput } from './request-params.js';
import { runVintedUserProfiles } from './run-user-profile.js';
import { ScrappaClient } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<VintedUserProfileInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const requests = buildVintedUserProfileRequests(input);
        console.log(`Running ${requests.length} Vinted user profile request(s)`);
        console.log(`First request: ${describeVintedUserProfileRequest(requests[0])}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const summary = await runVintedUserProfiles({
            actor: Actor,
            client,
            requests,
            attempts: SCRAPPA_MAX_ATTEMPTS,
        });

        console.log('Vinted user profiles completed');
        console.log('Results summary:', JSON.stringify(summary));

        if (summary.statusMessage) {
            await Actor.exit({ statusMessage: summary.statusMessage });
            return;
        }
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        console.error('Actor failed: ' + rawMessage);
        await Actor.fail(rawMessage);
        return;
    }

    await Actor.exit();
}

main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    process.exitCode = 1;
});

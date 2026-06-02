import { Actor } from 'apify';
import {
    actorChargingApi,
    getTranslationChargeLimitStatus,
    pushTranslationResult,
} from './charging.js';
import { buildTranslationRequests, describeTranslationRequests } from './input.js';
import type { GoogleTranslateInput } from './input.js';
import { runTranslations } from './run-translations.js';
import { describeScrappaError, ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 30000;

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<GoogleTranslateInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const requests = buildTranslationRequests(input);
        console.log(`Running ${describeTranslationRequests(requests)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const summary = await runTranslations(
            requests,
            client,
            {
                push: (item) => pushTranslationResult(actorChargingApi, item),
            },
            (processed, requested) => getTranslationChargeLimitStatus(actorChargingApi, processed, requested),
        );

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', requests.length === 1 && summary.firstItem
            ? summary.firstItem
            : {
                requested: summary.requested,
                succeeded: summary.succeeded,
                failed: summary.failed,
                saved: summary.saved,
                status_message: summary.statusMessage,
            });

        console.log(summary.statusMessage
            ? `Google Translate run completed: ${summary.statusMessage}`
            : 'Google Translate run completed successfully');
        console.log('Results summary:', JSON.stringify(summary, null, 2));

        if (summary.statusMessage) {
            await Actor.exit({ statusMessage: summary.statusMessage });
            return;
        }
    } catch (error) {
        const rawMessage = describeScrappaError(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Google Translate request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer batched translations or run the request again.`
            : rawMessage;
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main().catch((error) => {
    const message = describeScrappaError(error);
    console.error('Actor failed: ' + message);
    process.exitCode = 1;
});

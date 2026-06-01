import { Actor } from 'apify';
import {
    actorChargingApi,
    getDomainChargeLimitStatus,
    pushDomainResult,
} from './charging.js';
import { getDomainRequests, type DomainAvailabilityInput } from './input.js';
import {
    buildDomainAvailabilityDatasetItem,
    buildDomainAvailabilityFailureItem,
    isPerDomainAvailabilityFailure,
    type DomainAvailabilityDatasetItem,
    type DomainAvailabilityResponse,
} from './results.js';
import { ScrappaClient, describeScrappaError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 25000;
const SCRAPPA_MAX_ATTEMPTS = 3;

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<DomainAvailabilityInput>();
        const requests = getDomainRequests(input);
        if (requests.length === 0) {
            throw new Error('At least one domain is required. Provide domain or domains.');
        }

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        let firstResult: DomainAvailabilityDatasetItem | undefined;
        let succeeded = 0;
        let failed = 0;

        console.log(`Checking availability for ${requests.length} domain${requests.length === 1 ? '' : 's'}`);

        for (const request of requests) {
            const { input_domain: inputDomain, domain } = request;
            let result: DomainAvailabilityDatasetItem;

            if (!domain) {
                result = buildDomainAvailabilityFailureItem(
                    new Error(request.validation_error ?? 'Invalid domain'),
                    inputDomain,
                );
            } else {
                const statusMessage = getDomainChargeLimitStatus(actorChargingApi, succeeded + failed, requests.length);
                if (statusMessage) {
                    console.log(statusMessage);
                    await Actor.exit({ statusMessage });
                    return;
                }

                console.log(`Checking domain availability: ${domain}`);

                try {
                    const response = await client.get<DomainAvailabilityResponse>(
                        '/domains/availability',
                        { domain },
                        { attempts: SCRAPPA_MAX_ATTEMPTS },
                    );
                    result = buildDomainAvailabilityDatasetItem(response, inputDomain, domain);
                } catch (error) {
                    if (!isPerDomainAvailabilityFailure(error)) {
                        throw error;
                    }

                    console.warn(`Domain availability returned a per-domain failure for ${domain}: ${describeScrappaError(error)}`);
                    result = buildDomainAvailabilityFailureItem(error, inputDomain, domain);
                }
            }

            const pushResult = await pushDomainResult(actorChargingApi, result);
            if (!pushResult.saved) {
                await Actor.exit({ statusMessage: pushResult.statusMessage ?? 'Charge limit reached before saving all successful domain availability results.' });
                return;
            }

            firstResult ??= result;
            if (result.success) {
                succeeded += 1;
            } else {
                failed += 1;
            }
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', requests.length === 1 && firstResult
            ? firstResult
            : { requested: requests.length, succeeded, failed });

        console.log('Domain availability checks completed successfully');
        console.log('Results summary:', JSON.stringify({ requested: requests.length, succeeded, failed }, null, 2));
    } catch (error) {
        const message = describeScrappaError(error);
        console.error(`Actor failed: ${message}`);
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

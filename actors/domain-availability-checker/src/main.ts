import { Actor } from 'apify';
import { getDomainRequests, type DomainAvailabilityInput } from './input.js';
import { buildDomainAvailabilityParams } from './request-params.js';
import {
    buildDomainAvailabilityDatasetItem,
    buildDomainAvailabilityFailureItem,
    isRecoverableDomainAvailabilityError,
    type DomainAvailabilityDatasetItem,
    type DomainAvailabilityResponse,
} from './results.js';
import { ScrappaClient } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 25000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const DOMAIN_RESULT_CHARGE_EVENT = 'domain-result';

function describeUnknownError(error: unknown): string {
    if (error instanceof Error) {
        return error.message;
    }

    return typeof error === 'string' ? error : String(error);
}

async function pushResult(item: DomainAvailabilityDatasetItem): Promise<boolean> {
    const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent || !item.success) {
        await Actor.pushData(item);
        return true;
    }

    const chargeResult = await Actor.pushData(item, DOMAIN_RESULT_CHARGE_EVENT);
    if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < 1) {
        console.warn(`Charge limit reached before saving ${item.domain}; stopping batch without writing uncharged success results.`);
        return false;
    }

    return true;
}

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
                console.log(`Checking domain availability: ${domain}`);

                try {
                    const response = await client.get<DomainAvailabilityResponse>(
                        '/domains/availability',
                        buildDomainAvailabilityParams(domain),
                        { attempts: SCRAPPA_MAX_ATTEMPTS },
                    );
                    result = buildDomainAvailabilityDatasetItem(response, inputDomain, domain);
                } catch (error) {
                    if (!isRecoverableDomainAvailabilityError(error)) {
                        throw error;
                    }

                    console.warn(`Domain availability returned a per-domain failure for ${domain}: ${describeUnknownError(error)}`);
                    result = buildDomainAvailabilityFailureItem(error, inputDomain, domain);
                }
            }

            const pushed = await pushResult(result);
            if (!pushed) {
                await Actor.exit({ statusMessage: 'Charge limit reached before saving all successful domain availability results.' });
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
        const message = describeUnknownError(error);
        console.error(`Actor failed: ${message}`);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main();

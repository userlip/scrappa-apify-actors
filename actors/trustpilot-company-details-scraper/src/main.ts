import { Actor } from 'apify';
import { pushChargedItems } from './charging.js';
import {
    buildCompanyDetailsParams,
    buildTrustpilotCompanyDetailsPlan,
    describeTrustpilotCompanyDetailsRequest,
} from './request-params.js';
import type { TrustpilotCompanyDetailsInput } from './request-params.js';
import { buildTrustpilotCompanyDetailsDatasetItem } from './response-utils.js';
import type { TrustpilotCompanyDetailsResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;

function formatErrorMessage(error: unknown): string {
    const rawMessage = error instanceof Error ? error.message : String(error);
    return error instanceof ScrappaTimeoutError
        ? `${rawMessage}. The Trustpilot company details request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer domains or run the request again.`
        : rawMessage;
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<TrustpilotCompanyDetailsInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const plan = buildTrustpilotCompanyDetailsPlan(input);
        console.log(`Fetching Trustpilot company details for ${describeTrustpilotCompanyDetailsRequest(plan)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const responses: TrustpilotCompanyDetailsResponse[] = [];
        const failures: Record<string, string>[] = [];
        let savedCompanies = 0;
        let statusMessage: string | null = null;

        for (const companyDomain of plan.domains) {
            const params = buildCompanyDetailsParams(plan, companyDomain);
            console.log(`Fetching Trustpilot company details for ${companyDomain}`);

            try {
                const response = await client.get<TrustpilotCompanyDetailsResponse>('/trustpilot/company-details', params, {
                    attempts: SCRAPPA_MAX_ATTEMPTS,
                });
                responses.push(response);

                const item = buildTrustpilotCompanyDetailsDatasetItem(response, { companyDomain, params });
                const result = await pushChargedItems({
                    isPayPerEvent: () => Actor.getChargingManager().getPricingInfo().isPayPerEvent,
                    pushData: (items, eventName) => eventName === undefined
                        ? Actor.pushData(items)
                        : Actor.pushData(items, eventName),
                }, [item]);
                savedCompanies += result.savedCount;
                console.log(`Saved ${result.savedCount} Trustpilot company detail result(s) for ${companyDomain}`);

                if (result.statusMessage) {
                    statusMessage = result.statusMessage;
                    break;
                }
            } catch (error) {
                const message = formatErrorMessage(error);
                failures.push({ company_domain: companyDomain, error: message });
                console.error(`Failed to fetch Trustpilot company details for ${companyDomain}: ${message}`);
            }
        }

        if (!statusMessage && failures.length > 0) {
            statusMessage = `${failures.length} of ${plan.domains.length} Trustpilot company detail request(s) failed.`;
        }

        const output = {
            request: {
                endpoint: '/trustpilot/company-details',
                company_domains: plan.domains,
                ...plan.baseParams,
            },
            companies_requested: plan.domains.length,
            companies_saved: savedCompanies,
            companies_failed: failures.length,
            responses_saved: responses.length,
            status_message: statusMessage,
            failures,
            responses,
        };

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', output);

        console.log('Trustpilot company details extraction completed successfully');
        console.log('Results summary:', JSON.stringify({
            companies_requested: plan.domains.length,
            companies_saved: savedCompanies,
            companies_failed: failures.length,
        }));

        if (savedCompanies === 0 && failures.length > 0) {
            await Actor.fail(statusMessage ?? 'No Trustpilot company details were saved.');
            return;
        }

        if (statusMessage) {
            await Actor.exit({ statusMessage });
            return;
        }
    } catch (error) {
        const message = formatErrorMessage(error);
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

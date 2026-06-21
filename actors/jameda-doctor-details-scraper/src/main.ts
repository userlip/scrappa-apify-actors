import { Actor } from 'apify';
import { pushChargedItems } from './charging.js';
import {
    buildDoctorDetailsParams,
    buildJamedaDoctorDetailsPlan,
    describeJamedaDoctorDetailsRequest,
} from './request-params.js';
import type { JamedaDoctorDetailsInput } from './request-params.js';
import {
    buildJamedaDoctorDetailsDatasetItem,
    buildJamedaDoctorDetailsOutputSummary,
} from './response-utils.js';
import type { JamedaDoctorDetailsResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;

function formatErrorMessage(error: unknown): string {
    const rawMessage = error instanceof Error ? error.message : String(error);
    return error instanceof ScrappaTimeoutError
        ? `${rawMessage}. The Jameda doctor details request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer doctor URLs or run the request again.`
        : rawMessage;
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<JamedaDoctorDetailsInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const plan = buildJamedaDoctorDetailsPlan(input);
        console.log(`Fetching Jameda doctor details for ${describeJamedaDoctorDetailsRequest(plan)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const failures: Record<string, string>[] = [...plan.inputFailures];
        let savedProfiles = 0;
        let statusMessage: string | null = null;

        for (const doctorUrl of plan.doctorUrls) {
            const params = buildDoctorDetailsParams(doctorUrl);
            console.log(`Fetching Jameda doctor details for ${doctorUrl}`);

            try {
                const response = await client.get<JamedaDoctorDetailsResponse>('/jameda/doctor-details', params, {
                    attempts: SCRAPPA_MAX_ATTEMPTS,
                });

                const item = buildJamedaDoctorDetailsDatasetItem(response, { doctorUrl, params });
                const result = await pushChargedItems({
                    isPayPerEvent: () => Actor.getChargingManager().getPricingInfo().isPayPerEvent,
                    pushData: (items, eventName) => eventName === undefined
                        ? Actor.pushData(items)
                        : Actor.pushData(items, eventName),
                }, [item]);
                savedProfiles += result.savedCount;
                console.log(`Saved ${result.savedCount} Jameda doctor profile result(s) for ${doctorUrl}`);

                if (result.statusMessage) {
                    statusMessage = result.statusMessage;
                    break;
                }
            } catch (error) {
                const message = formatErrorMessage(error);
                failures.push({ doctor_url: doctorUrl, error: message });
                console.error(`Failed to fetch Jameda doctor details for ${doctorUrl}: ${message}`);
            }
        }

        if (!statusMessage && failures.length > 0) {
            statusMessage = `${failures.length} Jameda doctor detail request(s) failed; ${savedProfiles} profile(s) saved.`;
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', buildJamedaDoctorDetailsOutputSummary({
            doctorUrls: plan.doctorUrls,
            savedProfiles,
            failures,
            statusMessage,
        }));

        console.log('Jameda doctor details extraction completed successfully');
        console.log('Results summary:', JSON.stringify({
            doctors_requested: plan.doctorUrls.length,
            doctors_saved: savedProfiles,
            doctors_failed: failures.length,
        }));

        if (savedProfiles === 0 && failures.length > 0) {
            await Actor.fail(statusMessage ?? 'No Jameda doctor profiles were saved.');
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

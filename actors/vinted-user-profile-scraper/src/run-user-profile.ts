import {
    buildVintedUserProfileDatasetItem,
    getVintedUserProfile,
    type VintedUserProfileResponse,
} from './response-utils.js';
import {
    getVintedUserProfileAvailableChargeCount,
    getVintedUserProfileChargeLimitStatus,
    pushSuccessfulVintedUserProfile,
} from './charging.js';
import { isActorLevelScrappaFailure } from './failures.js';
import { PROFILE_REQUEST_CONCURRENCY } from './runtime-budget.js';
import { ScrappaTimeoutError } from './shared/scrappa-client.js';
import type { VintedUserProfileRequest } from './request-params.js';

export interface VintedUserProfileClient {
    get<T>(endpoint: string, params: Record<string, unknown>, options?: { attempts?: number }): Promise<T>;
}

export interface VintedUserProfileActor {
    getChargingManager(): {
        getPricingInfo(): { isPayPerEvent?: boolean };
        calculateMaxEventChargeCountWithinLimit(eventName: string): number;
    };
    pushData(data: unknown, eventName?: string): Promise<{ chargedCount?: number; eventChargeLimitReached?: boolean } | void>;
}

export interface VintedUserProfileRunOptions {
    actor: VintedUserProfileActor;
    client: VintedUserProfileClient;
    requests: VintedUserProfileRequest[];
    attempts: number;
}

export interface VintedUserProfileRunSummary {
    requested: number;
    succeeded: number;
    failed: number;
    statusMessage: string | null;
}

interface ProfileBatchState {
    actorLevelFailure: unknown | null;
}

export async function runVintedUserProfiles(options: VintedUserProfileRunOptions): Promise<VintedUserProfileRunSummary> {
    const { actor, client, requests, attempts } = options;
    let succeeded = 0;
    let failed = 0;
    let statusMessage: string | null = null;
    let nextRequestOffset = 0;

    while (nextRequestOffset < requests.length && !statusMessage) {
        const firstRequest = requests[nextRequestOffset];
        const chargeLimitStatus = getVintedUserProfileChargeLimitStatus(actor, succeeded, firstRequest.index);
        if (chargeLimitStatus) {
            statusMessage = chargeLimitStatus;
            console.log(chargeLimitStatus, JSON.stringify({
                event: 'user-profile-result',
                profiles_requested: requests.length,
                profiles_saved: succeeded,
                next_request_index: firstRequest.index,
            }));
            break;
        }

        const availableChargeCount = getVintedUserProfileAvailableChargeCount(actor);
        const batchSize = Math.min(
            PROFILE_REQUEST_CONCURRENCY,
            requests.length - nextRequestOffset,
            availableChargeCount ?? PROFILE_REQUEST_CONCURRENCY,
        );
        const batch = requests.slice(nextRequestOffset, nextRequestOffset + batchSize);
        nextRequestOffset += batch.length;
        const batchState: ProfileBatchState = { actorLevelFailure: null };

        const results = await Promise.allSettled(
            batch.map((request) => processVintedUserProfile({ actor, client, request, attempts, batchState })),
        );
        const actorLevelFailure = results.find(
            (result): result is PromiseRejectedResult => result.status === 'rejected' && isActorLevelScrappaFailure(result.reason),
        );
        if (batchState.actorLevelFailure !== null) {
            throw batchState.actorLevelFailure;
        }
        if (actorLevelFailure !== undefined) {
            throw actorLevelFailure.reason;
        }

        for (const settledResult of results) {
            if (settledResult.status === 'rejected') {
                throw settledResult.reason;
            }

            const result = settledResult.value;
            succeeded += result.succeeded;
            failed += result.failed;
            statusMessage ??= result.statusMessage;
        }
    }

    return { requested: requests.length, succeeded, failed, statusMessage };
}

async function processVintedUserProfile(options: {
    actor: VintedUserProfileActor;
    client: VintedUserProfileClient;
    request: VintedUserProfileRequest;
    attempts: number;
    batchState: ProfileBatchState;
}): Promise<{ succeeded: number; failed: number; statusMessage: string | null }> {
    const { actor, client, request, attempts, batchState } = options;
    console.log(`Fetching Vinted user profile ${request.userId} in ${String(request.params.country)}`);

    try {
        const response = await client.get<VintedUserProfileResponse>('/vinted/user-profile', request.params, { attempts });
        const profile = getVintedUserProfile(response);
        const item = buildVintedUserProfileDatasetItem(profile, request, response);

        // An auth failure in a sibling worker makes the run fail at actor level.
        // Do not create a dataset row or charge another result once that failure
        // has been observed; Promise.allSettled() still drains this worker.
        if (batchState.actorLevelFailure !== null) {
            console.warn(`Skipping Vinted user profile request ${request.index + 1} after an actor-level failure.`);
            return { succeeded: 0, failed: 1, statusMessage: null };
        }

        const pushResult = await pushSuccessfulVintedUserProfile(actor, item, request.index);

        if (pushResult.saved) {
            console.log(`Saved Vinted user profile result ${request.index + 1}`);
            return { succeeded: 1, failed: 0, statusMessage: pushResult.statusMessage };
        }

        if (pushResult.statusMessage) {
            console.log(pushResult.statusMessage, JSON.stringify({
                event: 'user-profile-result',
                charged_count: pushResult.chargedCount,
                requested_count: 1,
                request_index: request.index,
            }));
            return { succeeded: 0, failed: 0, statusMessage: pushResult.statusMessage };
        }

        console.warn(`Vinted user profile request ${request.index + 1} was not saved or charged.`);
        return { succeeded: 0, failed: 1, statusMessage: null };
    } catch (error) {
        if (isActorLevelScrappaFailure(error)) {
            batchState.actorLevelFailure ??= error;
            throw error;
        }

        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. Run the request again or check Scrappa availability.`
            : rawMessage;
        console.warn(`Vinted user profile request ${request.index + 1} failed: ${message}`);
        return { succeeded: 0, failed: 1, statusMessage: null };
    }
}

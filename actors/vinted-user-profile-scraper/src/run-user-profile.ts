import {
    buildVintedUserProfileDatasetItem,
    getVintedUserProfile,
    type VintedUserProfileResponse,
} from './response-utils.js';
import {
    getVintedUserProfileChargeLimitStatus,
    pushSuccessfulVintedUserProfile,
} from './charging.js';
import { isActorLevelScrappaFailure } from './failures.js';
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

export async function runVintedUserProfiles(options: VintedUserProfileRunOptions): Promise<VintedUserProfileRunSummary> {
    const { actor, client, requests, attempts } = options;
    let succeeded = 0;
    let failed = 0;
    let statusMessage: string | null = null;

    for (const request of requests) {
        const chargeLimitStatus = getVintedUserProfileChargeLimitStatus(actor, succeeded, request.index);
        if (chargeLimitStatus) {
            statusMessage = chargeLimitStatus;
            console.log(chargeLimitStatus, JSON.stringify({
                event: 'user-profile-result',
                profiles_requested: requests.length,
                profiles_saved: succeeded,
                next_request_index: request.index,
            }));
            break;
        }

        console.log(`Fetching Vinted user profile ${request.userId} in ${String(request.params.country)}`);

        try {
            const response = await client.get<VintedUserProfileResponse>('/vinted/user-profile', request.params, { attempts });
            const profile = getVintedUserProfile(response);
            const item = buildVintedUserProfileDatasetItem(profile, request, response);
            const pushResult = await pushSuccessfulVintedUserProfile(actor, item, request.index);

            if (pushResult.saved) {
                succeeded += 1;
                console.log(`Saved Vinted user profile result ${request.index + 1}`);
            }

            if (pushResult.statusMessage) {
                statusMessage = pushResult.statusMessage;
                console.log(statusMessage, JSON.stringify({
                    event: 'user-profile-result',
                    charged_count: pushResult.chargedCount,
                    requested_count: requests.length,
                    request_index: request.index,
                }));
                break;
            }
        } catch (error) {
            if (isActorLevelScrappaFailure(error)) {
                throw error;
            }
            failed += 1;
            const message = error instanceof Error ? error.message : String(error);
            console.warn(`Vinted user profile request ${request.index + 1} failed: ${message}`);
        }
    }

    return { requested: requests.length, succeeded, failed, statusMessage };
}

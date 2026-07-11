import type { TikTokChallengeDetailsRequest } from './request-params.js';
import { challengeId, challengeName, extractChallengeDetail, normalizeChallengeDetail } from './response-utils.js';
import type { TikTokChallengeDetailsResponse } from './response-utils.js';

export const CHALLENGE_DETAIL_CHARGE_EVENT = 'challenge-detail-result';

export interface BatchDependencies {
    getCapacity(): number;
    fetch(request: TikTokChallengeDetailsRequest): Promise<TikTokChallengeDetailsResponse>;
    save(item: Record<string, unknown>): Promise<{ savedCount: number; chargeLimitReached: boolean }>;
}

export interface ChallengeDetailOutcome {
    request_type: TikTokChallengeDetailsRequest['type'];
    request_value: string;
    status: 'saved' | 'failed' | 'not_attempted';
    error?: string;
}

export interface ChallengeDetailSummary {
    requested: number;
    attempted: number;
    succeeded: number;
    failed: number;
    saved: number;
    charge_limit_reached: boolean;
    status_message: string | null;
    outcomes: ChallengeDetailOutcome[];
}

function safeError(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
}

function addNotAttemptedOutcomes(
    requests: TikTokChallengeDetailsRequest[],
    startIndex: number,
    outcomes: ChallengeDetailOutcome[],
    error: string,
): void {
    for (const request of requests.slice(startIndex)) {
        outcomes.push({ request_type: request.type, request_value: request.value, status: 'not_attempted', error });
    }
}

export async function runChallengeDetailsBatch(requests: TikTokChallengeDetailsRequest[], dependencies: BatchDependencies): Promise<ChallengeDetailSummary> {
    const outcomes: ChallengeDetailOutcome[] = [];
    let attempted = 0;
    let saved = 0;
    let chargeLimitReached = false;
    let statusMessage: string | null = null;

    for (let index = 0; index < requests.length; index += 1) {
        const request = requests[index];
        if (dependencies.getCapacity() <= 0) {
            chargeLimitReached = true;
            statusMessage = 'Charge limit reached before fetching another TikTok challenge detail.';
            addNotAttemptedOutcomes(requests, index, outcomes, 'Charge limit reached');
            break;
        }

        attempted += 1;
        try {
            const response = await dependencies.fetch(request);
            if (response.code !== undefined && response.code !== 0) {
                throw new Error(`Scrappa TikTok Challenge Details API returned code ${response.code}: ${response.msg ?? 'Unknown error'}`);
            }
            const challenge = extractChallengeDetail(response.data);
            if (!challenge) throw new Error('Scrappa returned no challenge detail record');
            if (request.type === 'challenge_name') {
                const returnedName = challengeName(challenge);
                if (!returnedName || returnedName.toLowerCase() !== request.value.toLowerCase()) {
                    throw new Error(`Scrappa returned challenge ${JSON.stringify(returnedName)} but ${JSON.stringify(request.value)} was requested`);
                }
            } else {
                const returnedId = challengeId(challenge);
                if (returnedId !== request.value) {
                    throw new Error(`Scrappa returned challenge ID ${JSON.stringify(returnedId)} but ${JSON.stringify(request.value)} was requested`);
                }
            }

            const pushResult = await dependencies.save(normalizeChallengeDetail(challenge, request));
            if (pushResult.savedCount !== 1) {
                outcomes.push({ request_type: request.type, request_value: request.value, status: 'failed', error: 'Apify did not save a chargeable challenge detail result' });
                if (pushResult.chargeLimitReached) {
                    chargeLimitReached = true;
                    statusMessage = 'Charge limit reached while saving a TikTok challenge detail.';
                    addNotAttemptedOutcomes(requests, index + 1, outcomes, 'Charge limit reached');
                    break;
                }
                continue;
            }
            saved += 1;
            outcomes.push({ request_type: request.type, request_value: request.value, status: 'saved' });
            if (pushResult.chargeLimitReached) {
                chargeLimitReached = true;
                statusMessage = 'Charge limit reached after saving a TikTok challenge detail.';
                addNotAttemptedOutcomes(requests, index + 1, outcomes, 'Charge limit reached');
                break;
            }
        } catch (error) {
            outcomes.push({ request_type: request.type, request_value: request.value, status: 'failed', error: safeError(error) });
        }
    }

    return {
        requested: requests.length,
        attempted,
        succeeded: saved,
        failed: outcomes.filter((outcome) => outcome.status === 'failed').length,
        saved,
        charge_limit_reached: chargeLimitReached,
        status_message: statusMessage,
        outcomes,
    };
}

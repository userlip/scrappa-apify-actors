import type { DirectionsRequest } from './request-params.js';
import { buildDirectionsDatasetRows } from './response-utils.js';
import type { DirectionsResponse } from './response-utils.js';
import type { RouteWriter } from './charged-save.js';

const DIRECTIONS_ENDPOINT = '/maps/directions';
const MAX_ATTEMPTS = 3;
export const MAX_BATCH_DURATION_MS = 240_000;

export interface DirectionsClient {
    get<T>(
        endpoint: string,
        params: Record<string, string>,
        options: { attempts: number; deadlineAt?: number },
    ): Promise<T>;
}

export interface BatchFailure {
    requestIndex: number;
    origin: string;
    destination: string;
    message: string;
}

export interface BatchResult {
    requested: number;
    succeeded: number;
    failed: number;
    alternativesSaved: number;
    charged: number;
    failures: BatchFailure[];
    chargeLimitReached: boolean;
}

export interface BatchRunOptions {
    maxDurationMs?: number;
    now?: () => number;
}

function describeError(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
}

export async function runDirectionsBatch(
    requests: DirectionsRequest[],
    client: DirectionsClient,
    writer: RouteWriter,
    options: BatchRunOptions = {},
): Promise<BatchResult> {
    const failures: BatchFailure[] = [];
    let succeeded = 0;
    let alternativesSaved = 0;
    let charged = 0;
    const now = options.now ?? Date.now;
    const deadlineAt = now() + (options.maxDurationMs ?? MAX_BATCH_DURATION_MS);

    const addDeadlineFailures = (startIndex: number): void => {
        for (let index = startIndex; index < requests.length; index += 1) {
            const request = requests[index];
            failures.push({
                requestIndex: request.index,
                origin: request.origin,
                destination: request.destination,
                message: 'Batch deadline reached before this route could be processed',
            });
        }
    };

    for (let requestIndex = 0; requestIndex < requests.length; requestIndex += 1) {
        const request = requests[requestIndex];
        if (!writer.canSave()) {
            return {
                requested: requests.length,
                succeeded,
                failed: failures.length,
                alternativesSaved,
                charged,
                failures,
                chargeLimitReached: true,
            };
        }

        if (now() >= deadlineAt) {
            addDeadlineFailures(requestIndex);
            break;
        }

        try {
            const response = await client.get<DirectionsResponse>(DIRECTIONS_ENDPOINT, request.params, {
                attempts: MAX_ATTEMPTS,
                deadlineAt,
            });
            const rows = buildDirectionsDatasetRows(response, request);
            let requestSaved = 0;

            for (const row of rows) {
                const saveResult = await writer.save(row);
                if (saveResult.saved) {
                    requestSaved += 1;
                    alternativesSaved += 1;
                    charged += saveResult.chargedCount;
                }

                if (saveResult.chargeLimitReached) {
                    return {
                        requested: requests.length,
                        succeeded: succeeded + (requestSaved > 0 ? 1 : 0),
                        failed: failures.length,
                        alternativesSaved,
                        charged,
                        failures,
                        chargeLimitReached: true,
                    };
                }
            }

            if (requestSaved > 0) {
                succeeded += 1;
            }
        } catch (error) {
            failures.push({
                requestIndex: request.index,
                origin: request.origin,
                destination: request.destination,
                message: describeError(error),
            });
        }
    }

    return {
        requested: requests.length,
        succeeded,
        failed: failures.length,
        alternativesSaved,
        charged,
        failures,
        chargeLimitReached: false,
    };
}

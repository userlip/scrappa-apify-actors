export const JOB_RESULT_CHARGE_EVENT = 'job-result';

export interface PushChargedItemsResult {
    savedCount: number;
    statusMessage: string | null;
}

export interface ChargeResult {
    eventChargeLimitReached?: boolean;
    chargedCount: number;
}

export interface ChargedDataset {
    isPayPerEvent(): boolean;
    pushData(items: Record<string, unknown>[], eventName?: string): Promise<ChargeResult | void>;
}

export interface PushChargedItemsOptions {
    chargeEvent?: boolean;
}

export async function pushChargedItems(
    dataset: ChargedDataset,
    items: Record<string, unknown>[],
    options: PushChargedItemsOptions = {},
): Promise<PushChargedItemsResult> {
    if (items.length === 0) {
        return { savedCount: 0, statusMessage: null };
    }

    if (!dataset.isPayPerEvent() || options.chargeEvent === false) {
        await dataset.pushData(items);
        return { savedCount: items.length, statusMessage: null };
    }

    const chargeResult = await dataset.pushData(items, JOB_RESULT_CHARGE_EVENT) as ChargeResult;
    if (chargeResult.eventChargeLimitReached) {
        const savedCount = Math.min(chargeResult.chargedCount, items.length);
        const statusMessage = `Charge limit reached after saving ${savedCount} of ${items.length} LinkedIn job detail results.`;
        console.log(statusMessage, JSON.stringify({
            event: JOB_RESULT_CHARGE_EVENT,
            charged_count: chargeResult.chargedCount,
            requested_count: items.length,
            saved_count: savedCount,
        }));
        return { savedCount, statusMessage };
    }

    return { savedCount: items.length, statusMessage: null };
}

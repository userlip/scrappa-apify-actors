export interface ChargingManager {
    getPricingInfo(): { isPayPerEvent: boolean };
}

export interface PushDataResult {
    chargedCount: number;
    eventChargeLimitReached: boolean;
}

export interface DatasetWriter {
    pushData(item: Record<string, unknown>, eventName?: string): Promise<PushDataResult | void>;
}

export async function saveChallengeDetail(
    item: Record<string, unknown>,
    chargingManager: ChargingManager,
    dataset: DatasetWriter,
    eventName: string,
): Promise<{ savedCount: number; chargeLimitReached: boolean }> {
    if (!chargingManager.getPricingInfo().isPayPerEvent) {
        await dataset.pushData(item);
        return { savedCount: 1, chargeLimitReached: false };
    }

    const result = await dataset.pushData(item, eventName) as PushDataResult;
    return {
        savedCount: Math.min(result.chargedCount, 1),
        chargeLimitReached: result.eventChargeLimitReached || result.chargedCount < 1,
    };
}

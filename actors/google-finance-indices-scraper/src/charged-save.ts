export interface ChargingManager {
    getPricingInfo(): { isPayPerEvent: boolean };
}

export interface DatasetWriter {
    pushData(item: object, eventName?: string): Promise<{
        chargedCount: number;
        eventChargeLimitReached: boolean;
    } | void>;
}

export async function saveIndex(
    item: object,
    charging: ChargingManager,
    dataset: DatasetWriter,
    event = 'index-result',
): Promise<{ savedCount: number; chargeLimitReached: boolean }> {
    if (!charging.getPricingInfo().isPayPerEvent) {
        await dataset.pushData(item);
        return { savedCount: 1, chargeLimitReached: false };
    }

    const result = await dataset.pushData(item, event) as {
        chargedCount: number;
        eventChargeLimitReached: boolean;
    };

    return {
        savedCount: Math.min(result.chargedCount, 1),
        chargeLimitReached: result.eventChargeLimitReached || result.chargedCount < 1,
    };
}

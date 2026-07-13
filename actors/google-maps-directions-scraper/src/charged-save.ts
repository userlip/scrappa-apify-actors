export const ROUTE_RESULT_CHARGE_EVENT = 'route-result';

export interface ChargeResult {
    chargedCount?: number;
    eventChargeLimitReached?: boolean;
}

export interface ChargingManager {
    getPricingInfo(): { isPayPerEvent?: boolean };
    calculateMaxEventChargeCountWithinLimit(eventName: string): number;
}

export interface RouteDatasetWriter {
    pushData(item: Record<string, unknown>, eventName?: string): Promise<ChargeResult | void>;
}

export interface RouteSaveResult {
    saved: boolean;
    chargedCount: number;
    chargeLimitReached: boolean;
}

export interface RouteWriter {
    canSave(): boolean;
    save(item: Record<string, unknown>): Promise<RouteSaveResult>;
}

export function createChargedRouteWriter(
    charging: { getChargingManager(): ChargingManager },
    dataset: RouteDatasetWriter,
): RouteWriter {
    return {
        canSave(): boolean {
            const manager = charging.getChargingManager();
            if (!manager.getPricingInfo().isPayPerEvent) {
                return true;
            }

            return manager.calculateMaxEventChargeCountWithinLimit(ROUTE_RESULT_CHARGE_EVENT) > 0;
        },

        async save(item: Record<string, unknown>): Promise<RouteSaveResult> {
            const manager = charging.getChargingManager();
            if (!manager.getPricingInfo().isPayPerEvent) {
                await dataset.pushData(item);
                return { saved: true, chargedCount: 1, chargeLimitReached: false };
            }

            if (manager.calculateMaxEventChargeCountWithinLimit(ROUTE_RESULT_CHARGE_EVENT) < 1) {
                return { saved: false, chargedCount: 0, chargeLimitReached: true };
            }

            const result = await dataset.pushData(item, ROUTE_RESULT_CHARGE_EVENT);
            // Apify aggregates the explicit route-result charge with the
            // synthetic default-dataset-item charge for this write. Each
            // call stores one row, so only one route-result can be charged.
            const chargedCount = Math.min(result?.chargedCount ?? 0, 1);
            return {
                saved: chargedCount >= 1,
                chargedCount,
                chargeLimitReached: Boolean(result?.eventChargeLimitReached) || chargedCount < 1,
            };
        },
    };
}

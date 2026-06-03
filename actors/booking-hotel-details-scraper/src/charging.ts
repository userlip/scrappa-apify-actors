export const BOOKING_HOTEL_RESULT_CHARGE_EVENT = 'hotel-result';

interface ChargeResult {
    eventChargeLimitReached?: boolean;
    chargedCount?: number;
}

interface ActorLike {
    getChargingManager(): {
        getPricingInfo(): {
            isPayPerEvent?: boolean;
        };
    };
    pushData(data: unknown, eventName?: string): Promise<ChargeResult>;
}

export async function pushSuccessfulHotelItem(actor: ActorLike, item: Record<string, unknown>): Promise<ChargeResult> {
    const { isPayPerEvent } = actor.getChargingManager().getPricingInfo();

    if (isPayPerEvent) {
        return actor.pushData(item, BOOKING_HOTEL_RESULT_CHARGE_EVENT);
    }

    await actor.pushData(item);

    return {};
}

export async function pushErrorHotelItem(actor: Pick<ActorLike, 'pushData'>, item: Record<string, unknown>): Promise<void> {
    await actor.pushData(item);
}

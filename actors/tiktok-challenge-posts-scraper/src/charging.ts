export const RESULT_EVENT = 'challenge-post-result';

export interface ActorPort {
    getChargingManager(): {
        getPricingInfo(): { isPayPerEvent?: boolean };
        calculateMaxEventChargeCountWithinLimit(eventName: string): number;
    };
    pushData(data: unknown, eventName?: string): Promise<{ chargedCount?: number; eventChargeLimitReached?: boolean }>;
}

export function getAvailableCapacity(actor: ActorPort, requested: number): number {
    const manager = actor.getChargingManager();
    if (!manager.getPricingInfo().isPayPerEvent) {
        return requested;
    }
    return Math.min(requested, manager.calculateMaxEventChargeCountWithinLimit(RESULT_EVENT));
}

export async function pushVideos(actor: ActorPort, videos: Record<string, unknown>[]): Promise<{ saved: number; limitReached: boolean }> {
    if (videos.length === 0) {
        return { saved: 0, limitReached: false };
    }
    if (!actor.getChargingManager().getPricingInfo().isPayPerEvent) {
        await actor.pushData(videos);
        return { saved: videos.length, limitReached: false };
    }

    let limitReached = false;
    for (const video of videos) {
        const result = await actor.pushData(video, RESULT_EVENT);
        limitReached ||= Boolean(result.eventChargeLimitReached);
    }
    return {
        saved: videos.length,
        limitReached,
    };
}

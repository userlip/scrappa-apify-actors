export interface GoogleMapsBusinessDetailsInput {
    business_id?: string;
    business_ids?: string[];
    use_cache?: boolean;
    maximum_cache_age?: number;
}

export interface BusinessIdRequest {
    input_business_id: string;
    business_id: string;
}

export const MAX_BUSINESS_IDS_PER_RUN = 50;

export function getBusinessIdRequests(input: GoogleMapsBusinessDetailsInput | null): BusinessIdRequest[] {
    const rawBusinessIds = [
        ...(typeof input?.business_id === 'string' ? [input.business_id] : []),
        ...(Array.isArray(input?.business_ids) ? input.business_ids : []),
    ];

    const seen = new Set<string>();
    const requests: BusinessIdRequest[] = [];

    for (const rawBusinessId of rawBusinessIds) {
        const businessId = rawBusinessId.trim();
        if (!businessId || seen.has(businessId)) {
            continue;
        }

        seen.add(businessId);
        requests.push({
            input_business_id: businessId,
            business_id: businessId,
        });
    }

    if (requests.length > MAX_BUSINESS_IDS_PER_RUN) {
        throw new Error(`business_ids must contain ${MAX_BUSINESS_IDS_PER_RUN} unique items or fewer`);
    }

    return requests;
}

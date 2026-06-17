export interface TrustedShopsShopProfileInput {
    tsid?: unknown;
    tsids?: unknown;
    url?: unknown;
    urls?: unknown;
    include_raw_response?: unknown;
}

export interface TrustedShopsShopProfileRequest {
    tsid?: string;
    source_url?: string;
    validation_error?: string;
}

export interface TrustedShopsShopProfilePlan {
    requests: TrustedShopsShopProfileRequest[];
    includeRawResponse: boolean;
}

const TSID_PATTERN = /^[A-Z0-9]{33}$/;
const MAX_SHOPS_PER_RUN = 100;

interface InputValue {
    field: 'tsid' | 'tsids' | 'url' | 'urls';
    value: unknown;
}

function splitStringList(value: string): string[] {
    return value
        .split(/\r?\n|,/)
        .map((item) => item.trim())
        .filter(Boolean);
}

function collectValues(input: TrustedShopsShopProfileInput, field: InputValue['field']): InputValue[] {
    const value = input[field];
    if (value === undefined || value === null || value === '') {
        return [];
    }

    if (Array.isArray(value)) {
        return value.map((item) => ({ field, value: item }));
    }

    if (typeof value === 'string' && (field === 'tsids' || field === 'urls')) {
        return splitStringList(value).map((item) => ({ field, value: item }));
    }

    return [{ field, value }];
}

export function normalizeTrustedShopsTsid(value: unknown, field = 'tsid'): string {
    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const tsid = value.trim().toUpperCase();
    if (!TSID_PATTERN.test(tsid)) {
        throw new Error(`${field} must be a 33-character TrustedShops TSID using uppercase letters and numbers`);
    }

    return tsid;
}

export function extractTrustedShopsTsidFromUrl(value: unknown, field = 'url'): TrustedShopsShopProfileRequest {
    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const sourceUrl = value.trim();
    if (sourceUrl === '') {
        throw new Error(`${field} cannot be empty`);
    }

    const match = sourceUrl.match(/(?:info_|tsid=|tsID=)([A-Z0-9]{33})/i)
        ?? sourceUrl.match(/\b([A-Z0-9]{33})\b/i);

    if (!match) {
        throw new Error(`${field} must contain a TrustedShops TSID, for example https://www.trustedshops.de/bewertung/info_XFB15FFBDE1DEE7A55D292A7D48598A6A.html`);
    }

    return {
        tsid: normalizeTrustedShopsTsid(match[1], field),
        source_url: sourceUrl,
    };
}

function buildRequest(inputValue: InputValue): TrustedShopsShopProfileRequest {
    try {
        if (inputValue.field === 'tsid' || inputValue.field === 'tsids') {
            return { tsid: normalizeTrustedShopsTsid(inputValue.value, inputValue.field) };
        }

        return extractTrustedShopsTsidFromUrl(inputValue.value, inputValue.field);
    } catch (error) {
        return {
            source_url: typeof inputValue.value === 'string' ? inputValue.value.trim() : undefined,
            validation_error: error instanceof Error ? error.message : String(error),
        };
    }
}

export function buildTrustedShopsShopProfilePlan(
    input: TrustedShopsShopProfileInput,
): TrustedShopsShopProfilePlan {
    const values = [
        ...collectValues(input, 'tsid'),
        ...collectValues(input, 'tsids'),
        ...collectValues(input, 'url'),
        ...collectValues(input, 'urls'),
    ];

    if (values.length === 0) {
        throw new Error('Provide tsids or urls. Batch multiple TrustedShops merchants in one run whenever possible.');
    }

    const seen = new Set<string>();
    const requests: TrustedShopsShopProfileRequest[] = [];

    for (const value of values) {
        const request = buildRequest(value);
        const key = request.tsid ?? `invalid:${request.source_url ?? request.validation_error ?? requests.length}`;
        if (seen.has(key)) {
            continue;
        }

        seen.add(key);
        requests.push(request);
    }

    if (requests.length > MAX_SHOPS_PER_RUN) {
        throw new Error(`A single run can include at most ${MAX_SHOPS_PER_RUN} TrustedShops shops`);
    }

    return {
        requests,
        includeRawResponse: input.include_raw_response === true,
    };
}

export function describeTrustedShopsShopProfileRequest(plan: TrustedShopsShopProfilePlan): string {
    if (plan.requests.length === 1) {
        return plan.requests[0].tsid ?? '1 invalid input';
    }

    return `${plan.requests.length} TrustedShops shop profile inputs`;
}

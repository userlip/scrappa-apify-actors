export interface JamedaReviewsInput {
    doctor_url?: unknown;
    doctor_urls?: unknown;
    page?: unknown;
    sort?: unknown;
    rating?: unknown;
    per_page?: unknown;
}

export interface JamedaReviewsPlan {
    doctorUrls: string[];
    inputFailures: Record<string, string>[];
    page: number;
    sort?: JamedaReviewsSort;
    rating?: string;
    perPage: number;
}

export type JamedaReviewsSort = 'newest' | 'oldest' | 'highest' | 'lowest';

const JAMEDA_HOST = 'www.jameda.de';
const JAMEDA_BASE_URL = `https://${JAMEDA_HOST}`;
const MAX_DOCTOR_URLS_PER_RUN = 100;
const MAX_PAGE = 500;
const DEFAULT_PAGE = 1;
const DEFAULT_PER_PAGE = 20;
const MAX_PER_PAGE = 100;
const SORT_VALUES = new Set<JamedaReviewsSort>(['newest', 'oldest', 'highest', 'lowest']);

interface DoctorUrlInputValue {
    field: 'doctor_url' | 'doctor_urls';
    value: unknown;
}

function normalizePath(pathname: string): string {
    const normalized = pathname.replace(/\/{2,}/g, '/').replace(/\/+$/, '');
    return normalized.startsWith('/') ? normalized : `/${normalized}`;
}

function hasJamedaReviewProfileShape(pathname: string): boolean {
    const segments = pathname.split('/').filter(Boolean);
    if (segments[0] === 'gesundheitseinrichtungen') {
        return segments.length >= 2;
    }

    return segments.length >= 3;
}

function looksLikeHostStyleUrl(value: string): boolean {
    const firstSegment = value.split('/')[0];
    return firstSegment.includes('.');
}

export function cleanJamedaDoctorUrl(value: unknown, field = 'doctor_url'): string {
    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const rawValue = value.trim();
    if (rawValue === '') {
        throw new Error(`${field} cannot be empty`);
    }

    let url: URL;
    try {
        if (/^https?:\/\//i.test(rawValue)) {
            url = new URL(rawValue);
        } else if (looksLikeHostStyleUrl(rawValue)) {
            url = new URL(`https://${rawValue}`);
        } else {
            const path = rawValue.startsWith('/') ? rawValue : `/${rawValue}`;
            url = new URL(path, JAMEDA_BASE_URL);
        }
    } catch {
        throw new Error(`${field} must be a valid Jameda doctor URL or path`);
    }

    if (!/^jameda\.de$/i.test(url.hostname.replace(/^www\./i, ''))) {
        throw new Error(`${field} must use the jameda.de domain`);
    }

    const pathname = normalizePath(url.pathname);
    if (pathname === '/' || !hasJamedaReviewProfileShape(pathname)) {
        throw new Error(`${field} must point to a Jameda doctor profile path like /markus-lietzau-msc/zahnarzt/berlin or a supported /gesundheitseinrichtungen facility path`);
    }

    return `${JAMEDA_BASE_URL}${pathname}${url.hash}`;
}

function parseDoctorUrlInputs(input: JamedaReviewsInput): DoctorUrlInputValue[] {
    const values: DoctorUrlInputValue[] = [];

    if (input.doctor_url !== undefined && input.doctor_url !== null && input.doctor_url !== '') {
        values.push({ field: 'doctor_url', value: input.doctor_url });
    }

    if (input.doctor_urls !== undefined && input.doctor_urls !== null && input.doctor_urls !== '') {
        if (Array.isArray(input.doctor_urls)) {
            values.push(...input.doctor_urls.map((value) => ({ field: 'doctor_urls' as const, value })));
        } else if (typeof input.doctor_urls === 'string') {
            values.push(...input.doctor_urls
                .split(/\r?\n|,/)
                .map((value) => value.trim())
                .filter(Boolean)
                .map((value) => ({ field: 'doctor_urls' as const, value })));
        } else {
            throw new Error('doctor_urls must be an array of strings or a comma/newline-separated string');
        }
    }

    return values;
}

function cleanInteger(value: unknown, field: string, min: number, max: number): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    const normalized = typeof value === 'string' && value.trim() !== '' && /^\d+$/.test(value.trim())
        ? Number(value.trim())
        : value;

    if (typeof normalized !== 'number' || !Number.isInteger(normalized)) {
        throw new Error(`${field} must be an integer`);
    }

    if (normalized < min || normalized > max) {
        throw new Error(`${field} must be between ${min} and ${max}`);
    }

    return normalized;
}

function cleanSort(value: unknown): JamedaReviewsSort | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'string') {
        throw new Error('sort must be a string');
    }

    const normalized = value.trim().toLowerCase();
    if (!SORT_VALUES.has(normalized as JamedaReviewsSort)) {
        throw new Error('sort must be one of newest, oldest, highest, lowest');
    }

    return normalized as JamedaReviewsSort;
}

export function cleanRatingFilter(value: unknown): string | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    const values = Array.isArray(value) ? value : String(value).split(',');
    const ratings = values.map((item) => String(item).trim()).filter(Boolean);
    if (ratings.length === 0) {
        return undefined;
    }

    for (const rating of ratings) {
        if (!/^[1-5]$/.test(rating)) {
            throw new Error('rating must contain only values from 1 to 5');
        }
    }

    return [...new Set(ratings)].join(',');
}

export function buildJamedaReviewsPlan(input: JamedaReviewsInput): JamedaReviewsPlan {
    const values = parseDoctorUrlInputs(input);
    if (values.length === 0) {
        throw new Error('Provide doctor_urls or doctor_url');
    }

    const doctorUrls: string[] = [];
    const inputFailures: Record<string, string>[] = [];

    for (const { field, value } of values) {
        try {
            doctorUrls.push(cleanJamedaDoctorUrl(value, field));
        } catch (error) {
            inputFailures.push({
                doctor_url: typeof value === 'string' ? value : String(value),
                error: error instanceof Error ? error.message : String(error),
            });
        }
    }

    const uniqueDoctorUrls = [...new Set(doctorUrls)];
    if (uniqueDoctorUrls.length === 0) {
        throw new Error('No valid Jameda doctor URLs were provided');
    }

    if (uniqueDoctorUrls.length > MAX_DOCTOR_URLS_PER_RUN) {
        throw new Error(`doctor_urls can include at most ${MAX_DOCTOR_URLS_PER_RUN} doctor URLs per run`);
    }

    return {
        doctorUrls: uniqueDoctorUrls,
        inputFailures,
        page: cleanInteger(input.page, 'page', 1, MAX_PAGE) ?? DEFAULT_PAGE,
        sort: cleanSort(input.sort),
        rating: cleanRatingFilter(input.rating),
        perPage: cleanInteger(input.per_page, 'per_page', 1, MAX_PER_PAGE) ?? DEFAULT_PER_PAGE,
    };
}

export function buildJamedaReviewsParams(plan: JamedaReviewsPlan, doctorUrl: string): Record<string, unknown> {
    return {
        doctor_url: doctorUrl,
        page: plan.page,
        per_page: plan.perPage,
        ...(plan.sort ? { sort: plan.sort } : {}),
        ...(plan.rating ? { rating: plan.rating } : {}),
    };
}

export function describeJamedaReviewsRequest(plan: JamedaReviewsPlan): string {
    const urlDescription = plan.doctorUrls.length === 1 ? plan.doctorUrls[0] : `${plan.doctorUrls.length} doctor URLs`;
    const filters = [
        `page ${plan.page}`,
        `${plan.perPage} per page`,
        plan.sort ? `sort ${plan.sort}` : null,
        plan.rating ? `rating ${plan.rating}` : null,
    ].filter(Boolean).join(', ');

    return `${urlDescription} (${filters})`;
}

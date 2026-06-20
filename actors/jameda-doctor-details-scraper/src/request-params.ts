export interface JamedaDoctorDetailsInput {
    doctorUrl?: unknown;
    doctorUrls?: unknown;
}

export interface JamedaDoctorDetailsPlan {
    doctorUrls: string[];
    inputFailures: Record<string, string>[];
}

const JAMEDA_HOST = 'www.jameda.de';
const JAMEDA_BASE_URL = `https://${JAMEDA_HOST}`;
const MAX_DOCTOR_URLS_PER_RUN = 100;

interface DoctorUrlInputValue {
    field: 'doctorUrl' | 'doctorUrls';
    value: unknown;
}

function normalizePath(pathname: string): string {
    const normalized = pathname.replace(/\/{2,}/g, '/').replace(/\/+$/, '');
    return normalized.startsWith('/') ? normalized : `/${normalized}`;
}

function hasDoctorProfileShape(pathname: string): boolean {
    const segments = pathname.split('/').filter(Boolean);
    return segments.length >= 3;
}

export function cleanJamedaDoctorUrl(value: unknown, field = 'doctorUrl'): string {
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
    if (pathname === '/' || !hasDoctorProfileShape(pathname)) {
        throw new Error(`${field} must point to a Jameda doctor profile path, for example /markus-lietzau-msc/zahnarzt/berlin`);
    }

    return `${JAMEDA_BASE_URL}${pathname}`;
}

function parseDoctorUrlInputs(input: JamedaDoctorDetailsInput): DoctorUrlInputValue[] {
    const values: DoctorUrlInputValue[] = [];

    if (input.doctorUrl !== undefined && input.doctorUrl !== null && input.doctorUrl !== '') {
        values.push({ field: 'doctorUrl', value: input.doctorUrl });
    }

    if (input.doctorUrls !== undefined && input.doctorUrls !== null && input.doctorUrls !== '') {
        if (Array.isArray(input.doctorUrls)) {
            values.push(...input.doctorUrls.map((value) => ({ field: 'doctorUrls' as const, value })));
        } else if (typeof input.doctorUrls === 'string') {
            values.push(...input.doctorUrls
                .split(/\r?\n|,/)
                .map((value) => value.trim())
                .filter(Boolean)
                .map((value) => ({ field: 'doctorUrls' as const, value })));
        } else {
            throw new Error('doctorUrls must be an array of strings or a comma/newline-separated string');
        }
    }

    return values;
}

export function buildJamedaDoctorDetailsPlan(input: JamedaDoctorDetailsInput): JamedaDoctorDetailsPlan {
    const values = parseDoctorUrlInputs(input);
    if (values.length === 0) {
        throw new Error('Provide doctorUrls or doctorUrl');
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
        throw new Error(`doctorUrls can include at most ${MAX_DOCTOR_URLS_PER_RUN} doctor URLs per run`);
    }

    return { doctorUrls: uniqueDoctorUrls, inputFailures };
}

export function buildDoctorDetailsParams(doctorUrl: string): Record<string, unknown> {
    return {
        doctor_url: doctorUrl,
    };
}

export function describeJamedaDoctorDetailsRequest(plan: JamedaDoctorDetailsPlan): string {
    if (plan.doctorUrls.length === 1) {
        return plan.doctorUrls[0];
    }

    return `${plan.doctorUrls.length} doctor URLs`;
}

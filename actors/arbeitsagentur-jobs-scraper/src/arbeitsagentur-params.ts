export interface ArbeitsagenturJobsInput {
    was?: string;
    wo?: string;
    berufsfeld?: string;
    arbeitgeber?: string;
    angebotsart?: number;
    arbeitszeit?: string;
    befristung?: number;
    veroeffentlichtseit?: number;
    umkreis?: number;
    zeitarbeit?: boolean;
    pav?: boolean;
    page?: number;
    size?: number;
}

export const DEFAULT_ARBEITSAGENTUR_JOBS_QUERY = 'Software Entwickler';

export const DEFAULT_ARBEITSAGENTUR_JOBS_INPUT: ArbeitsagenturJobsInput = {
    was: DEFAULT_ARBEITSAGENTUR_JOBS_QUERY,
    wo: 'Berlin',
    umkreis: 25,
    page: 1,
    size: 25,
};

const KNOWN_INPUT_KEYS = new Set<keyof ArbeitsagenturJobsInput>([
    'was',
    'wo',
    'berufsfeld',
    'arbeitgeber',
    'angebotsart',
    'arbeitszeit',
    'befristung',
    'veroeffentlichtseit',
    'umkreis',
    'zeitarbeit',
    'pav',
    'page',
    'size',
]);

export function normalizeArbeitsagenturJobsInput(input?: ArbeitsagenturJobsInput | null): ArbeitsagenturJobsInput {
    if (!input) {
        return { ...DEFAULT_ARBEITSAGENTUR_JOBS_INPUT };
    }

    const normalized = normalizeFields(input);
    const hasKnownInput = Object.entries(normalized).some(([key, value]) => {
        return KNOWN_INPUT_KEYS.has(key as keyof ArbeitsagenturJobsInput) && value !== undefined && value !== '';
    });

    if (!hasKnownInput) {
        return { ...DEFAULT_ARBEITSAGENTUR_JOBS_INPUT };
    }

    return {
        ...DEFAULT_ARBEITSAGENTUR_JOBS_INPUT,
        ...normalized,
        was: normalized.was ?? DEFAULT_ARBEITSAGENTUR_JOBS_QUERY,
    };
}

export function buildArbeitsagenturJobsParams(input: ArbeitsagenturJobsInput): Record<string, unknown> {
    const params: Record<string, unknown> = {};
    const entries: [keyof ArbeitsagenturJobsInput, unknown][] = [
        ['was', input.was],
        ['wo', input.wo],
        ['berufsfeld', input.berufsfeld],
        ['arbeitgeber', input.arbeitgeber],
        ['angebotsart', input.angebotsart],
        ['arbeitszeit', input.arbeitszeit],
        ['befristung', input.befristung],
        ['veroeffentlichtseit', input.veroeffentlichtseit],
        ['umkreis', input.umkreis],
        ['zeitarbeit', input.zeitarbeit],
        ['pav', input.pav],
        ['page', input.page],
        ['size', input.size],
    ];

    for (const [key, value] of entries) {
        if (value !== undefined) {
            params[key] = value;
        }
    }

    return params;
}

function normalizeFields(input: ArbeitsagenturJobsInput): ArbeitsagenturJobsInput {
    const normalized: ArbeitsagenturJobsInput = {};

    for (const [key, value] of Object.entries(input) as [keyof ArbeitsagenturJobsInput, unknown][]) {
        if (!KNOWN_INPUT_KEYS.has(key)) {
            continue;
        }

        if (typeof value === 'string') {
            const trimmed = value.trim();
            if (trimmed !== '') {
                normalized[key] = normalizeStringValue(key, trimmed) as never;
            }
        } else if (value !== undefined && value !== null) {
            normalized[key] = value as never;
        }
    }

    return normalized;
}

function normalizeStringValue(key: keyof ArbeitsagenturJobsInput, value: string): string {
    if (key === 'arbeitszeit') {
        return value.toLowerCase().replace(/\s+/g, '');
    }

    return value;
}

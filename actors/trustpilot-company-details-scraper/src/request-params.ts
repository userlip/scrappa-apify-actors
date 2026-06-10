export interface TrustpilotCompanyDetailsInput {
    company_domain?: unknown;
    company_domains?: unknown;
    locale?: unknown;
    fields?: unknown;
}

export interface TrustpilotCompanyDetailsPlan {
    domains: string[];
    baseParams: Record<string, unknown>;
}

// Keep in sync with .actor/input_schema.json locale enum.
const LOCALES = [
    'da-DK',
    'de-AT',
    'de-CH',
    'de-DE',
    'en-AU',
    'en-CA',
    'en-GB',
    'en-IE',
    'en-NZ',
    'en-US',
    'es-ES',
    'fi-FI',
    'fr-BE',
    'nl-BE',
    'fr-FR',
    'it-IT',
    'ja-JP',
    'nb-NO',
    'nl-NL',
    'pl-PL',
    'pt-BR',
    'pt-PT',
    'sv-SE',
] as const;

const DEFAULT_LOCALE = 'en-US';
const MAX_DOMAINS_PER_RUN = 100;

interface DomainInputValue {
    field: 'company_domain' | 'company_domains';
    value: unknown;
}

function isValidDomainName(domain: string): boolean {
    if (domain.length > 253 || !domain.includes('.')) {
        return false;
    }

    const labels = domain.split('.');
    if (labels.some((label) => label.length === 0 || label.length > 63)) {
        return false;
    }

    const labelPattern = /^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$/;
    if (!labels.every((label) => labelPattern.test(label))) {
        return false;
    }

    return /^[a-z]{2,}$/.test(labels[labels.length - 1]);
}

function cleanDomain(value: unknown, field = 'company_domain'): string {
    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const rawValue = value.trim();
    if (rawValue === '') {
        throw new Error(`${field} cannot be empty`);
    }

    let domain: string;
    try {
        const url = new URL(/^https?:\/\//i.test(rawValue) ? rawValue : `https://${rawValue}`);
        domain = url.hostname.toLowerCase().replace(/^www\./i, '');
    } catch {
        throw new Error(`${field} must be a valid domain name, for example trustpilot.com`);
    }

    if (!isValidDomainName(domain)) {
        throw new Error(`${field} must be a valid domain name, for example trustpilot.com`);
    }

    return domain;
}

function cleanOptionalString(value: unknown, field: string, maxLength: number): string | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const trimmed = value.trim();
    if (trimmed === '') {
        return undefined;
    }

    if (trimmed.length > maxLength) {
        throw new Error(`${field} must be ${maxLength} characters or fewer`);
    }

    return trimmed;
}

function cleanEnum<T extends readonly string[]>(
    value: unknown,
    field: string,
    allowedValues: T,
): T[number] | undefined {
    const cleaned = cleanOptionalString(value, field, 100);
    if (cleaned === undefined) {
        return undefined;
    }

    if (!allowedValues.includes(cleaned)) {
        throw new Error(`${field} must be one of: ${allowedValues.join(', ')}`);
    }

    return cleaned;
}

function parseCompanyDomains(input: TrustpilotCompanyDetailsInput): string[] {
    const values: DomainInputValue[] = [];

    if (input.company_domain !== undefined && input.company_domain !== null && input.company_domain !== '') {
        values.push({ field: 'company_domain', value: input.company_domain });
    }

    if (input.company_domains !== undefined && input.company_domains !== null && input.company_domains !== '') {
        if (Array.isArray(input.company_domains)) {
            values.push(...input.company_domains.map((value) => ({ field: 'company_domains' as const, value })));
        } else if (typeof input.company_domains === 'string') {
            values.push(...input.company_domains
                .split(/\r?\n|,/)
                .map((value) => value.trim())
                .filter(Boolean)
                .map((value) => ({ field: 'company_domains' as const, value })));
        } else {
            throw new Error('company_domains must be an array of strings or a comma/newline-separated string');
        }
    }

    if (values.length === 0) {
        throw new Error('Provide company_domain or company_domains');
    }

    const domains = values.map(({ field, value }) => cleanDomain(value, field));
    const uniqueDomains = [...new Set(domains)];

    if (uniqueDomains.length > MAX_DOMAINS_PER_RUN) {
        throw new Error(`company_domains can include at most ${MAX_DOMAINS_PER_RUN} domains per run`);
    }

    return uniqueDomains;
}

export function buildTrustpilotCompanyDetailsPlan(input: TrustpilotCompanyDetailsInput): TrustpilotCompanyDetailsPlan {
    const domains = parseCompanyDomains(input);
    const baseParams: Record<string, unknown> = {
        locale: cleanEnum(input.locale, 'locale', LOCALES) ?? DEFAULT_LOCALE,
    };

    const fields = cleanOptionalString(input.fields, 'fields', 500);
    if (fields !== undefined) {
        baseParams.fields = fields;
    }

    return { domains, baseParams };
}

export function buildCompanyDetailsParams(
    plan: TrustpilotCompanyDetailsPlan,
    companyDomain: string,
): Record<string, unknown> {
    return {
        company_domain: companyDomain,
        ...plan.baseParams,
    };
}

export function describeTrustpilotCompanyDetailsRequest(plan: TrustpilotCompanyDetailsPlan): string {
    if (plan.domains.length === 1) {
        return plan.domains[0];
    }

    return `${plan.domains.length} company domains`;
}

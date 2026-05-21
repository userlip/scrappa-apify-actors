export interface SimilarwebTrafficInput {
    domain?: unknown;
    domains?: unknown;
}

export interface SimilarwebTrafficRequest {
    domain: string;
    inputDomain: string;
}

const MAX_DOMAINS_PER_RUN = 100;

function cleanInputString(value: unknown, field: string): string | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const trimmed = value.trim();
    return trimmed === '' ? undefined : trimmed;
}

function normalizeDomain(value: string): string {
    let domain = value.trim().toLowerCase();

    try {
        const parsed = new URL(domain.includes('://') ? domain : `https://${domain}`);
        domain = parsed.hostname;
    } catch {
        domain = domain.replace(/^https?:\/\//, '').replace(/\/.*$/, '');
    }

    domain = domain.replace(/^www\./, '').replace(/\.$/, '');

    if (/^\d{1,3}(?:\.\d{1,3}){3}$/.test(domain)) {
        throw new Error(`domain "${value}" must be a domain name, not an IP address`);
    }

    if (domain.length < 3) {
        throw new Error(`domain "${value}" must be at least 3 characters after normalization`);
    }

    if (domain.length > 253) {
        throw new Error(`domain "${value}" cannot exceed 253 characters after normalization`);
    }

    const label = '[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?';
    const domainPattern = new RegExp(`^${label}(?:\\.${label})+$`, 'i');
    if (!domainPattern.test(domain)) {
        throw new Error(`domain "${value}" must be a valid domain name such as example.com`);
    }

    return domain;
}

function getRawDomains(input: SimilarwebTrafficInput): string[] {
    const rawDomains: string[] = [];
    const singleDomain = cleanInputString(input.domain, 'domain');

    if (singleDomain !== undefined) {
        rawDomains.push(singleDomain);
    }

    if (input.domains !== undefined && input.domains !== null && input.domains !== '') {
        if (!Array.isArray(input.domains)) {
            throw new Error('domains must be an array of strings');
        }

        input.domains.forEach((value, index) => {
            const domain = cleanInputString(value, `domains[${index}]`);
            if (domain !== undefined) {
                rawDomains.push(domain);
            }
        });
    }

    return rawDomains;
}

export function buildSimilarwebTrafficRequests(input: SimilarwebTrafficInput): SimilarwebTrafficRequest[] {
    const rawDomains = getRawDomains(input);
    if (rawDomains.length === 0) {
        throw new Error('At least one domain is required. Provide domain or domains.');
    }

    const seen = new Set<string>();
    const requests: SimilarwebTrafficRequest[] = [];

    for (const inputDomain of rawDomains) {
        const domain = normalizeDomain(inputDomain);
        if (seen.has(domain)) {
            continue;
        }

        seen.add(domain);
        requests.push({ domain, inputDomain });
    }

    if (requests.length > MAX_DOMAINS_PER_RUN) {
        throw new Error(`domains cannot contain more than ${MAX_DOMAINS_PER_RUN} unique domains per run`);
    }

    return requests;
}

export function describeSimilarwebTrafficRequests(requests: SimilarwebTrafficRequest[]): string {
    if (requests.length === 1) {
        return requests[0].domain;
    }

    return `${requests.length} domains (${requests.slice(0, 3).map((request) => request.domain).join(', ')}${requests.length > 3 ? ', ...' : ''})`;
}

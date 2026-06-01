export interface DomainAvailabilityInput {
    domain?: unknown;
    domains?: unknown;
}

export interface DomainRequest {
    input_domain: string;
    domain?: string;
    validation_error?: string;
}

function cleanDomainInput(value: unknown, field: string): string | undefined {
    if (value === undefined || value === null) {
        return undefined;
    }

    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const trimmed = value.trim();
    return trimmed === '' ? undefined : trimmed;
}

function normalizeDomain(value: string): string {
    const original = value;
    let domain = value.trim();

    if (/^[a-z][a-z0-9+.-]*:\/\//i.test(domain)) {
        const url = new URL(domain);
        domain = url.hostname;
    } else {
        domain = domain.split('/', 1)[0]?.split('?', 1)[0]?.split('#', 1)[0] ?? domain;
    }

    domain = domain.trim().replace(/\.+$/u, '').toLowerCase();

    if (!domain.includes('.')) {
        throw new Error(`Invalid domain "${original}". Provide a fully qualified domain such as example.com.`);
    }

    if (domain.length > 253) {
        throw new Error(`Invalid domain "${original}". Domain names must be 253 characters or fewer.`);
    }

    const labels = domain.split('.');
    if (labels.some((label) => label.length < 1 || label.length > 63)) {
        throw new Error(`Invalid domain "${original}". Domain labels must be between 1 and 63 characters.`);
    }

    if (!labels.every((label) => /^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$/iu.test(label))) {
        throw new Error(`Invalid domain "${original}". Provide a valid fully qualified domain name.`);
    }

    return domain;
}

export function getDomainRequests(input: DomainAvailabilityInput | null): DomainRequest[] {
    const rawDomains: unknown[] = [];
    const singleDomain = cleanDomainInput(input?.domain, 'domain');
    if (singleDomain !== undefined) {
        rawDomains.push(singleDomain);
    }

    if (input?.domains !== undefined) {
        if (!Array.isArray(input.domains)) {
            throw new Error('domains must be an array of strings');
        }

        rawDomains.push(...input.domains);
    }

    const seen = new Set<string>();
    const requests: DomainRequest[] = [];

    rawDomains.forEach((rawDomain, index) => {
        let inputDomain: string | undefined;

        try {
            inputDomain = cleanDomainInput(rawDomain, `domains[${index}]`);
            if (inputDomain === undefined) {
                return;
            }

            const domain = normalizeDomain(inputDomain);
            if (seen.has(domain)) {
                return;
            }

            seen.add(domain);
            requests.push({ input_domain: inputDomain, domain });
        } catch (error) {
            const fallback = typeof inputDomain === 'string' ? inputDomain : String(rawDomain);
            const key = `invalid:${fallback}`;
            if (seen.has(key)) {
                return;
            }

            seen.add(key);
            requests.push({
                input_domain: fallback,
                validation_error: error instanceof Error ? error.message : String(error),
            });
        }
    });

    return requests;
}

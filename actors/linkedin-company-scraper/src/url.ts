export function normalizeLinkedInCompanyUrl(rawUrl: string): string {
    const candidate = rawUrl.trim();
    const withProtocol = /^[a-z]+:\/\//i.test(candidate) ? candidate : `https://${candidate}`;
    const parsed = new URL(withProtocol);

    parsed.hostname = parsed.hostname
        .replace(/^[a-z]{2,3}\.linkedin\.com$/i, 'www.linkedin.com')
        .replace(/^linkedin\.com$/i, 'www.linkedin.com')
        .replace(/^m\.linkedin\.com$/i, 'www.linkedin.com');
    parsed.search = '';
    parsed.hash = '';
    parsed.pathname = parsed.pathname.replace(/\/+$/, '');

    const companyMatch = parsed.pathname.match(/^\/company\/([a-zA-Z0-9._-]+)(?:\/.*)?$/i);
    if (!companyMatch) {
        throw new Error('Invalid LinkedIn company URL. Expected format: https://www.linkedin.com/company/company-slug');
    }

    parsed.pathname = `/company/${companyMatch[1]}`;

    return parsed.toString();
}

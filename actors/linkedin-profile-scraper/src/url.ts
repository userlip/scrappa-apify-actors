export function normalizeLinkedInProfileUrl(rawUrl: string): string {
    const candidate = rawUrl.trim();
    const withProtocol = /^[a-z]+:\/\//i.test(candidate) ? candidate : `https://${candidate}`;
    const parsed = new URL(withProtocol);

    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
        throw new Error('Invalid LinkedIn profile URL. Expected format: https://www.linkedin.com/in/profile-slug');
    }

    if (parsed.username || parsed.password || parsed.port) {
        throw new Error('Invalid LinkedIn profile URL. Expected format: https://www.linkedin.com/in/profile-slug');
    }

    parsed.hostname = parsed.hostname.replace(/^(?:(?:www|m|[a-z]{2,3})\.)?linkedin\.com$/i, 'www.linkedin.com');
    if (parsed.hostname !== 'www.linkedin.com') {
        throw new Error('Invalid LinkedIn profile URL. Expected format: https://www.linkedin.com/in/profile-slug');
    }

    parsed.search = '';
    parsed.hash = '';
    parsed.pathname = parsed.pathname.replace(/\/+$/, '');

    const profileMatch = parsed.pathname.match(/^\/in\/([a-zA-Z0-9._-]+)(?:\/.*)?$/i);
    if (!profileMatch) {
        throw new Error('Invalid LinkedIn profile URL. Expected format: https://www.linkedin.com/in/profile-slug');
    }

    parsed.pathname = `/in/${profileMatch[1]}`;

    return parsed.toString();
}

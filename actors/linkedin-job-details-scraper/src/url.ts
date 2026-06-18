const LINKEDIN_JOB_URL_MESSAGE = 'Invalid LinkedIn job URL. Expected format: https://www.linkedin.com/jobs/view/job-id';

export function normalizeLinkedInJobUrl(rawUrl: string): string {
    const candidate = rawUrl.trim();
    if (candidate === '') {
        throw new Error(LINKEDIN_JOB_URL_MESSAGE);
    }

    const withProtocol = /^[a-z]+:\/\//i.test(candidate) ? candidate : `https://${candidate}`;
    const parsed = new URL(withProtocol);

    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
        throw new Error(LINKEDIN_JOB_URL_MESSAGE);
    }

    if (parsed.username || parsed.password || parsed.port) {
        throw new Error(LINKEDIN_JOB_URL_MESSAGE);
    }

    parsed.hostname = parsed.hostname.replace(/^(?:(?:www|m|[a-z]{2,3})\.)?linkedin\.com$/i, 'www.linkedin.com');
    if (parsed.hostname !== 'www.linkedin.com') {
        throw new Error(LINKEDIN_JOB_URL_MESSAGE);
    }

    const jobMatch = parsed.pathname.match(/^\/jobs\/view\/([^/?#]+)(?:\/.*)?$/i);
    if (!jobMatch) {
        throw new Error(LINKEDIN_JOB_URL_MESSAGE);
    }

    parsed.pathname = `/jobs/view/${jobMatch[1]}`;
    parsed.search = '';
    parsed.hash = '';

    return parsed.toString();
}

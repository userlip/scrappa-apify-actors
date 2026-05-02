export function getScrappaApiKey(env = process.env) {
    const apiKey = env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
    }

    return apiKey;
}

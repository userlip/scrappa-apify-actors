export class ScrappaClient {
    constructor(
        private readonly apiKey: string,
        private readonly baseUrl = 'https://scrappa.co/api',
        private readonly timeoutMs = 60_000,
    ) {}

    async get<T>(path: string, params: Record<string, unknown>): Promise<T> {
        const url = new URL(`${this.baseUrl}${path}`);
        for (const [key, value] of Object.entries(params)) url.searchParams.set(key, String(value));
        const response = await fetch(url, {
            headers: { Accept: 'application/json', 'X-API-Key': this.apiKey },
            signal: AbortSignal.timeout(this.timeoutMs),
        });
        if (!response.ok) {
            throw new Error(`Scrappa API request failed with HTTP ${response.status}`);
        }
        return response.json() as Promise<T>;
    }
}

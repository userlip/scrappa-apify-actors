export class ScrappaTimeoutError extends Error {
    constructor(timeoutMs: number, options?: ErrorOptions) { super(`Scrappa API request timed out after ${timeoutMs}ms`, options); this.name = 'ScrappaTimeoutError'; }
}
export class ScrappaHttpError extends Error {
    constructor(readonly status: number) { super(`Scrappa API error (${status})`); this.name = 'ScrappaHttpError'; }
}
export function isRetryableScrappaError(error: unknown): boolean {
    if (error instanceof ScrappaTimeoutError) return true;
    if (error instanceof ScrappaHttpError) return [408, 429, 500, 502, 503, 504].includes(error.status);
    return error instanceof TypeError && /fetch failed|failed to fetch|network|terminated|reset|econnrefused|econnreset|socket hang up|chunk/i.test(error.message);
}
export class ScrappaClient {
    constructor(private readonly config: { apiKey: string; baseUrl?: string; timeoutMs?: number }) {}
    async get<T>(endpoint: string, params: object, attempts = 3): Promise<T> {
        let lastError: unknown;
        for (let attempt = 1; attempt <= attempts; attempt += 1) try {
            const url = new URL(`${this.config.baseUrl ?? 'https://scrappa.co/api'}${endpoint}`);
            for (const [key, value] of Object.entries(params)) if (value != null && value !== '') url.searchParams.set(key, String(value));
            const controller = new AbortController(); const timer = setTimeout(() => controller.abort(), this.config.timeoutMs ?? 60000);
            try {
                const response = await fetch(url, { headers: { 'X-API-Key': this.config.apiKey, Accept: 'application/json' }, signal: controller.signal });
                if (!response.ok) throw new ScrappaHttpError(response.status);
                return await response.json() as T;
            } catch (error) {
                if (error instanceof Error && error.name === 'AbortError') throw new ScrappaTimeoutError(this.config.timeoutMs ?? 60000, { cause: error });
                throw error;
            } finally { clearTimeout(timer); }
        } catch (error) { lastError = error; if (attempt === attempts || !isRetryableScrappaError(error)) break; await new Promise((resolve) => setTimeout(resolve, attempt * 250)); }
        throw lastError instanceof Error ? lastError : new Error('Scrappa API request failed');
    }
}

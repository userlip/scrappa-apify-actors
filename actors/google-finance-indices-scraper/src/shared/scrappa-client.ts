export class ScrappaClient {
    constructor(private readonly config: { apiKey: string; baseUrl?: string; timeoutMs?: number }) {}
    async get<T>(endpoint: string, params: object, attempts = 3): Promise<T> {
        let lastError: unknown;
        for (let attempt = 1; attempt <= attempts; attempt += 1) try {
            const url = new URL(`${this.config.baseUrl ?? 'https://scrappa.co/api'}${endpoint}`);
            for (const [key, value] of Object.entries(params)) if (value != null && value !== '') url.searchParams.set(key, String(value));
            const controller = new AbortController(); const timer = setTimeout(() => controller.abort(), this.config.timeoutMs ?? 60000);
            try { const response = await fetch(url, { headers: { 'X-API-Key': this.config.apiKey, Accept: 'application/json' }, signal: controller.signal }); if (!response.ok) throw Object.assign(new Error(`Scrappa API error (${response.status})`), { status: response.status }); return await response.json() as T; } finally { clearTimeout(timer); }
        } catch (error) { lastError = error; const status = (error as { status?: number }).status; if (attempt === attempts || (status && status < 429) || (status && status < 500)) break; await new Promise((resolve) => setTimeout(resolve, attempt * 250)); }
        throw lastError instanceof Error ? lastError : new Error('Scrappa API request failed');
    }
}

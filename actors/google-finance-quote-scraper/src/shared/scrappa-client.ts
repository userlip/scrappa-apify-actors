export interface ScrappaConfig {
    apiKey: string;
    baseUrl?: string;
    debug?: boolean;
    timeoutMs?: number;
}

export class ScrappaClient {
    private apiKey: string;
    private baseUrl: string;
    private debug: boolean;
    private timeoutMs: number;

    constructor(config: ScrappaConfig) {
        this.apiKey = config.apiKey;
        this.baseUrl = config.baseUrl ?? 'https://scrappa.co/api';
        this.debug = config.debug ?? false;
        this.timeoutMs = config.timeoutMs ?? 60000;
    }

    async get<T>(endpoint: string, params: Record<string, unknown> = {}): Promise<T> {
        return this.request<T>('GET', endpoint, params);
    }

    private async request<T>(
        method: 'GET',
        endpoint: string,
        params: Record<string, unknown> = {},
    ): Promise<T> {
        const url = new URL(`${this.baseUrl}${endpoint}`);

        Object.entries(params).forEach(([key, value]) => {
            if (value !== undefined && value !== null && value !== '') {
                url.searchParams.set(key, String(value));
            }
        });

        const headers: Record<string, string> = {
            'X-API-Key': this.apiKey,
            'Accept': 'application/json',
        };

        if (this.debug) {
            console.log(`[Scrappa] ${method} ${url.toString()}`);
        }

        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.timeoutMs);

        try {
            const response = await fetch(url.toString(), {
                method,
                headers,
                signal: controller.signal,
            });

            if (!response.ok) {
                let errorMessage: string;
                const responseClone = response.clone();

                try {
                    const errorData = await response.json() as { message?: string; errors?: Record<string, string[]> };
                    errorMessage = errorData.message ?? `HTTP ${response.status}`;

                    if (errorData.errors) {
                        const details = Object.entries(errorData.errors)
                            .map(([field, messages]) => `${field}: ${messages.join(', ')}`)
                            .join('; ');
                        errorMessage += ` - ${details}`;
                    }
                } catch (parseError) {
                    if (parseError instanceof Error && parseError.name === 'AbortError') {
                        throw parseError;
                    }
                    errorMessage = await responseClone.text() || `HTTP ${response.status}`;
                }

                throw new Error(`Scrappa API error (${response.status}): ${errorMessage}`);
            }

            return response.json() as Promise<T>;
        } catch (error) {
            if (error instanceof Error && error.name === 'AbortError') {
                throw new Error(`Scrappa API request timed out after ${this.timeoutMs}ms`);
            }
            throw error;
        } finally {
            clearTimeout(timeoutId);
        }
    }
}

export interface ScrappaConfig {
    apiKey: string;
    baseUrl?: string;
    debug?: boolean;
}

export interface ScrappaError {
    status: number;
    message: string;
    errors?: Record<string, string[]>;
}

export class ScrappaClient {
    private apiKey: string;
    private baseUrl: string;
    private debug: boolean;

    constructor(config: ScrappaConfig) {
        this.apiKey = config.apiKey;
        this.baseUrl = config.baseUrl ?? 'https://scrappa.co/api';
        this.debug = config.debug ?? false;
    }

    async get<T>(endpoint: string, params: Record<string, unknown> = {}): Promise<T> {
        return this.request<T>('GET', endpoint, params);
    }

    async post<T>(endpoint: string, body: Record<string, unknown> = {}): Promise<T> {
        return this.request<T>('POST', endpoint, {}, body);
    }

    private async request<T>(
        method: 'GET' | 'POST',
        endpoint: string,
        params: Record<string, unknown> = {},
        body?: Record<string, unknown>
    ): Promise<T> {
        const url = new URL(`${this.baseUrl}${endpoint}`);

        // Add query params for GET requests
        if (method === 'GET') {
            Object.entries(params).forEach(([key, value]) => {
                if (value !== undefined && value !== null && value !== '') {
                    // Skip false booleans - Laravel rejects use_cache=0
                    if (typeof value === 'boolean') {
                        if (value) {
                            url.searchParams.set(key, '1');
                        }
                    } else {
                        url.searchParams.set(key, String(value));
                    }
                }
            });
        }

        const headers: Record<string, string> = {
            'X-API-Key': this.apiKey,
            'Accept': 'application/json',
        };

        const options: RequestInit = {
            method,
            headers,
        };

        if (method === 'POST' && body) {
            headers['Content-Type'] = 'application/json';
            options.body = JSON.stringify(body);
        }

        if (this.debug) {
            console.log(`[Scrappa] ${method} ${url.toString()}`);
        }

        const response = await fetch(url.toString(), options);

        if (!response.ok) {
            let errorMessage: string;
            try {
                const errorData = await response.json() as { message?: string; errors?: Record<string, string[]> };
                errorMessage = errorData.message ?? `HTTP ${response.status}`;

                if (errorData.errors) {
                    const errorDetails = Object.entries(errorData.errors)
                        .map(([field, messages]) => `${field}: ${messages.join(', ')}`)
                        .join('; ');
                    errorMessage += ` - ${errorDetails}`;
                }
            } catch {
                errorMessage = await response.text() || `HTTP ${response.status}`;
            }

            throw new Error(`Scrappa API error (${response.status}): ${errorMessage}`);
        }

        return response.json() as Promise<T>;
    }
}

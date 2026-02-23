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
            const errorMessage = await this.readErrorMessage(response);

            const err = new Error(`Scrappa API error (${response.status}): ${errorMessage}`);
            (err as any).statusCode = response.status;
            throw err;
        }

        return response.json() as Promise<T>;
    }

    private async readErrorMessage(response: Response): Promise<string> {
        const fallback = response.statusText || `HTTP ${response.status}`;

        let bodyText: string;
        try {
            bodyText = await response.text();
        } catch {
            return fallback;
        }

        if (!bodyText) {
            return fallback;
        }

        const jsonMessage = this.tryParseJsonError(bodyText, fallback);
        if (jsonMessage) {
            return jsonMessage;
        }

        return bodyText.replace(/\s+/g, ' ').trim().slice(0, 500);
    }

    private tryParseJsonError(bodyText: string, fallback: string): string | null {
        try {
            const errorData = JSON.parse(bodyText) as { message?: string; errors?: Record<string, string[]> };
            let message = errorData.message ?? fallback;
            if (errorData.errors) {
                const errorDetails = Object.entries(errorData.errors)
                    .map(([field, messages]) => `${field}: ${messages.join(', ')}`)
                    .join('; ');
                if (errorDetails) {
                    message += ` - ${errorDetails}`;
                }
            }
            return message;
        } catch {
            return null;
        }
    }
}

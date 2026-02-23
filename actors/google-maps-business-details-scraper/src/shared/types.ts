export interface BaseActorInput {
    /**
     * Your Scrappa API key from https://scrappa.co
     */
    apiKey: string;

    /**
     * Optional: Override the Scrappa API base URL (for testing)
     */
    baseUrl?: string;
}

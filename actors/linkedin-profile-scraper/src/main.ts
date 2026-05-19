import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';
import { buildLinkedInProfileParams } from './request-params.js';
import { normalizeLinkedInProfileUrl } from './url.js';

interface LinkedInProfileInput {
    url?: string;
    urls?: string[];
    use_cache?: boolean;
    maximum_cache_age?: number;
}

interface Experience {
    company?: string;
    url?: string;
    start_date?: string;
    end_date?: string;
    [key: string]: unknown;
}

type LinkedInProfileResult = LinkedInProfileResponse & { url?: string };

interface Education {
    school?: string;
    url?: string;
    start_date?: string;
    end_date?: string;
    [key: string]: unknown;
}

interface Article {
    title?: string;
    url?: string;
    published_date?: string;
    [key: string]: unknown;
}

interface Activity {
    type?: string;
    text?: string;
    date?: string;
    [key: string]: unknown;
}

interface Publication {
    title?: string;
    publisher?: string;
    date?: string;
    [key: string]: unknown;
}

interface Project {
    title?: string;
    description?: string;
    date?: string;
    [key: string]: unknown;
}

interface Recommendation {
    name?: string;
    title?: string;
    text?: string;
    [key: string]: unknown;
}

interface SimilarProfile {
    name?: string;
    url?: string;
    title?: string;
    [key: string]: unknown;
}

interface LinkedInProfileResponse {
    success: boolean;
    name?: string;
    image?: string;
    location?: string;
    followers?: number;
    connections?: number;
    about?: string;
    job_titles?: string[];
    experience?: Experience[];
    education?: Education[];
    skills?: string[];
    articles?: Article[];
    activity?: Activity[];
    publications?: Publication[];
    projects?: Project[];
    recommendations?: Recommendation[];
    similar_profiles?: SimilarProfile[];
    cached?: boolean;
    cached_at?: string;
    message?: string;
    status_code?: number;
    [key: string]: unknown;
}

function getInputUrls(input: LinkedInProfileInput | null): string[] {
    const urls = [
        ...(input?.url ? [input.url] : []),
        ...(Array.isArray(input?.urls) ? input.urls : []),
    ]
        .map((url) => url.trim())
        .filter((url) => url.length > 0)
        .map(normalizeLinkedInProfileUrl);

    return [...new Set(urls)];
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        // Get API key from environment variable (set as Apify secret)
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<LinkedInProfileInput>();
        const urls = getInputUrls(input);
        if (urls.length === 0) {
            throw new Error('At least one LinkedIn profile URL is required. Provide url or urls.');
        }

        const client = new ScrappaClient({ apiKey });
        const output: LinkedInProfileResult[] = [];

        console.log(`Scraping ${urls.length} LinkedIn profile URL${urls.length === 1 ? '' : 's'}`);

        for (const normalizedUrl of urls) {
            console.log(`Fetching LinkedIn profile: ${normalizedUrl}`);

            const params = buildLinkedInProfileParams({
                url: normalizedUrl,
                use_cache: input?.use_cache,
                maximum_cache_age: input?.maximum_cache_age,
            });

            let result: LinkedInProfileResult;

            try {
                const response = await client.get<LinkedInProfileResponse>('/linkedin/profile', params);
                result = {
                    url: normalizedUrl,
                    ...response,
                };
            } catch (error) {
                const message = error instanceof Error ? error.message : String(error);

                // Handle 404s gracefully per URL so one missing profile does not stop the batch.
                if (message.includes('(404)')) {
                    console.warn(`Profile not found (404): ${normalizedUrl}`);
                    result = {
                        success: false,
                        url: normalizedUrl,
                        message: 'Profile not found',
                        status_code: 404,
                    };
                } else {
                    throw error;
                }
            }

            // Push the entire profile as a single dataset item
            if (result.success) {
                console.log(`Successfully scraped profile: ${result.name || 'Unknown'}`);
            } else if (result.status_code !== 404) {
                console.warn('Profile scraping returned success: false');
            }

            await Actor.pushData(result);
            output.push(result);
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', output.length === 1 ? output[0] : output);

        // Log summary
        console.log('LinkedIn profile scraping completed');

        const summary = {
            requested: urls.length,
            succeeded: output.filter((result) => result.success).length,
            failed: output.filter((result) => !result.success).length,
        };

        console.log('Profile summary:', JSON.stringify(summary, null, 2));

    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error(`Actor failed: ${message}`);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main();

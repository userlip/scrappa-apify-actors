import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';
import { buildLinkedInCompanyParams } from './request-params.js';
import { normalizeLinkedInCompanyUrl } from './url.js';

interface LinkedInCompanyInput {
    url?: string;
    urls?: string[];
    use_cache?: boolean;
    maximum_cache_age?: number;
}

interface Employee {
    name: string;
    title: string;
    profile_url: string;
}

interface Post {
    text: string;
    date: string;
    likes: number;
    comments: number;
}

interface Location {
    name: string;
    type: string;
}

interface SimilarPage {
    name: string;
    url: string;
}

interface Funding {
    round: string;
    amount: string;
    date: string;
}

interface Address {
    street?: string;
    city?: string;
    state?: string;
    country?: string;
    postal_code?: string;
}

interface LinkedInCompanyResponse {
    success: boolean;
    name?: string;
    description?: string;
    logo?: string;
    website?: string;
    employee_count?: number;
    address?: Address[];
    posts?: Post[];
    followers?: number;
    similar_pages?: SimilarPage[];
    specialties?: string[];
    employees?: Employee[];
    funding?: Funding | null;
    industry?: string;
    size?: string;
    type?: string;
    cached?: boolean;
    cached_at?: string;
    message?: string;
    status_code?: number;
}

type LinkedInCompanyResult = LinkedInCompanyResponse & { url?: string };

function getInputUrls(input: LinkedInCompanyInput | null): string[] {
    const urls = [
        ...(input?.url ? [input.url] : []),
        ...(Array.isArray(input?.urls) ? input.urls : []),
    ]
        .map((url) => url.trim())
        .filter((url) => url.length > 0);

    return [...new Set(urls)];
}

async function persistActorResult(result: LinkedInCompanyResult): Promise<void> {
    await Actor.pushData(result);
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        // Get API key from environment variable (set as Apify secret)
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<LinkedInCompanyInput>();
        const urls = getInputUrls(input);
        if (urls.length === 0) {
            throw new Error('At least one LinkedIn company URL is required. Provide url or urls.');
        }

        const client = new ScrappaClient({ apiKey });
        const output: LinkedInCompanyResult[] = [];

        console.log('Scraping ' + urls.length + ' LinkedIn company URL' + (urls.length === 1 ? '' : 's'));

        for (const rawUrl of urls) {
            const normalizedUrl = normalizeLinkedInCompanyUrl(rawUrl);
            console.log('Scraping LinkedIn company: "' + normalizedUrl + '"');
            if (normalizedUrl !== rawUrl) {
                console.log('(normalized from: ' + rawUrl + ')');
            }

            const params = buildLinkedInCompanyParams({
                url: normalizedUrl,
                use_cache: input?.use_cache,
                maximum_cache_age: input?.maximum_cache_age,
            });

            let result: LinkedInCompanyResult;

            try {
                const response = await client.get<LinkedInCompanyResponse>('/linkedin/company', params);
                result = {
                    url: normalizedUrl,
                    ...response,
                };
            } catch (error) {
                const message = error instanceof Error ? error.message : String(error);

                // Handle 404s gracefully - push a failure result instead of failing the actor
                if (message.includes('(404)')) {
                    console.log('Company not found (404): ' + normalizedUrl);
                    result = { success: false, url: normalizedUrl, message: 'Company not found', status_code: 404 };
                } else {
                    throw error;
                }
            }

            if (!result.success) {
                // Keep the run green when Scrappa returns a structured failure payload.
                if (result.status_code === 404) {
                    console.log('Company not found: ' + normalizedUrl);
                }

                console.warn('Company scraping returned success: false' + (result.message ? ` (${result.message})` : ''));
            } else {
                console.log('Successfully scraped company: ' + (result.name ?? 'Unknown'));
            }

            await persistActorResult(result);
            output.push(result);
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', output.length === 1 ? output[0] : output);

        // Log summary
        console.log('LinkedIn Company scrape completed successfully');

        const summary = {
            requested: urls.length,
            succeeded: output.filter((result) => result.success).length,
            failed: output.filter((result) => !result.success).length,
        };

        console.log('Results summary:', JSON.stringify(summary, null, 2));

    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main();

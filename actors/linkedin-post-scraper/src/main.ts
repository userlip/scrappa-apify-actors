import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';

interface LinkedInPostInput {
    url: string;
    use_cache?: boolean;
    maximum_cache_age?: number;
}

interface Author {
    name?: string;
    url?: string;
    image?: string;
    headline?: string;
}

interface Reactions {
    total?: number;
    likes?: number;
    comments?: number;
}

interface Comment {
    author?: string;
    text?: string;
    date?: string;
    [key: string]: unknown;
}

interface Article {
    title?: string;
    url?: string;
    author?: string;
    [key: string]: unknown;
}

interface LinkedInPostResponse {
    success: boolean;
    title?: string;
    url?: string;
    date_published?: string;
    date_modified?: string;
    image?: string;
    body?: string;
    author?: Author;
    reactions?: Reactions;
    topics?: string[];
    more_articles?: Article[];
    comments?: Comment[];
    [key: string]: unknown;
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        // Get API key from environment variable (set as Apify secret)
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<LinkedInPostInput>();
        if (!input?.url) {
            throw new Error('LinkedIn post URL is required');
        }

        console.log(`Scraping LinkedIn post: "${input.url}"`);

        const client = new ScrappaClient({ apiKey });

        const params: Record<string, unknown> = {
            url: input.url,
            maximum_cache_age: input.maximum_cache_age,
        };

        if (input.use_cache) {
            params.use_cache = 1;
        }

        let response: LinkedInPostResponse;
        try {
            response = await client.get<LinkedInPostResponse>('/linkedin/post', params);
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);

            // Handle 404 errors gracefully - push to dataset instead of failing
            if (message.includes('(404)')) {
                console.warn(`404 Not Found for URL: ${input.url}`);
                await Actor.pushData({
                    success: false,
                    error: message,
                    url: input.url,
                });
                await Actor.exit();
                return;
            }

            throw error;
        }

        // Check if the API response indicates failure
        if (!response.success) {
            console.warn(`API returned success: false for URL: ${input.url}`);
        }

        // Transform response for dataset view
        const datasetItem = {
            title: response.title,
            url: response.url,
            date_published: response.date_published,
            date_modified: response.date_modified,
            image: response.image,
            body: response.body,
            author_name: response.author?.name,
            author_url: response.author?.url,
            author_image: response.author?.image,
            author_headline: response.author?.headline,
            reactions_total: response.reactions?.total,
            reactions_likes: response.reactions?.likes,
            comments_count: response.reactions?.comments,
            topics: response.topics,
            more_articles: response.more_articles,
            comments: response.comments,
            success: response.success,
        };

        // Push the entire response as a single dataset item
        await Actor.pushData(datasetItem);
        console.log(`Successfully scraped LinkedIn post: "${response.title ?? 'Unknown'}"`);

        // Store full response in key-value store for complete data access
        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        // Log summary
        console.log('LinkedIn Post scraping completed successfully');

        const summary = {
            title: response.title,
            author: response.author?.name,
            date_published: response.date_published,
            reactions_total: response.reactions?.total ?? 0,
            comments_count: response.reactions?.comments ?? 0,
            topics_count: response.topics?.length ?? 0,
            more_articles_count: response.more_articles?.length ?? 0,
        };

        console.log('Results summary:', JSON.stringify(summary));

    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main();

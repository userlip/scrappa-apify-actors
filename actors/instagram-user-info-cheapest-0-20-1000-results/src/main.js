import { Actor } from 'apify';
import {
    fetchInstagramUser,
    getUsernames,
    runInstagramUserBatch,
} from './instagram-users.js';

Actor.main(async () => {
    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        await Actor.fail('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        return;
    }

    try {
        const usernames = getUsernames(await Actor.getInput());
        console.log(`Fetching Instagram user info for ${usernames.length} username${usernames.length === 1 ? '' : 's'}.`);

        const summary = await runInstagramUserBatch(
            usernames,
            (username) => fetchInstagramUser(username, apiKey),
            (items) => Actor.pushData(items),
        );

        console.log(`Instagram user batch completed: ${summary.succeeded} succeeded, ${summary.failed} failed.`);
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        await Actor.fail(message);
    }
});

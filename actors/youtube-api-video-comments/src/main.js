import { Actor } from 'apify';
import axios from 'axios';

async function searchVideoComments(id, sort, continuation = '') {
    // Validate that the required query parameter is present.
    if (!id) {
        throw new Error('Search query "id" not provided. Please provide a value for "id" in the input.');
    }
    
    // Construct the base API URL with required parameters
    let apiUrl = `https://ytapi.scrappa.co/videos/comments?id=${encodeURIComponent(id)}`;
    
    // Add optional parameters only if they have valid values
    if (sort && typeof sort === 'string' && sort.trim() !== '') {
        apiUrl += `&sort=${encodeURIComponent(sort)}`;
    }

    if (continuation && typeof continuation === 'string' && continuation.trim() !== '') {
        apiUrl += `&continuation=${encodeURIComponent(continuation)}`;
    }

    try {
        console.log(`Fetching from: ${apiUrl}`);
        const response = await axios.get(apiUrl);
        const data = response.data.comments;
        
        // Save the fetched data to the default dataset.
        await Actor.pushData(data);
        console.log(`Successfully fetched ${data.length} videos for query: ${id}`);
        
        // Log if there's a continuation token for next page
        if (response.data.continuation) {
            console.log(`Continuation token available for next page: ${response.data.continuation}`);
        }
    } catch (error) {
        console.log()
        console.error(`Failed to fetch videos for query: ${id}`, error.message);
        throw error;
    }
}

// Main Actor logic
Actor.main(async () => {
    // The init() call configures the Actor for its environment.
    await Actor.init();

    const input = await Actor.getInput();
    const { id, sort, continuation } = input;

    // Directly call the function with the input, as there is only one possible task.
    await searchVideoComments(id, sort, continuation);

    // Gracefully exit the Actor process.
    await Actor.exit();
});
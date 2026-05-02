import { Actor } from 'apify';
import axios from 'axios';

async function searchVideoDetail(id) {
    // Validate that the required query parameter is present.
    if (!id) {
        throw new Error('Search query "id" not provided. Please provide a value for "id" in the input.');
    }
    
    // Construct the base API URL with required parameters
    let apiUrl = `https://ytapi.scrappa.co/videos?id=${encodeURIComponent(id)}`;

    try {
        console.log(`Fetching from: ${apiUrl}`);
        const response = await axios.get(apiUrl);
        const data = response.data;
        
        // Save the fetched data to the default dataset.
        await Actor.pushData(data);
        console.log(`Successfully fetched ${data.length} id for query: ${id}`);
        
        // Log if there's a continuation token for next page
        if (response.data.continuation) {
            console.log(`Continuation token available for next page: ${response.data.continuation}`);
        }
    } catch (error) {
        console.error(`Failed to fetch id for query: ${id}`, error.message);
        throw error;
    }
}

// Main Actor logic
Actor.main(async () => {
    // The init() call configures the Actor for its environment.
    await Actor.init();

    const input = await Actor.getInput();
    const { id } = input;

    // Directly call the function with the input, as there is only one possible task.
    await searchVideoDetail(id);

    // Gracefully exit the Actor process.
    await Actor.exit();
});
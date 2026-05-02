import { Actor } from 'apify';
import axios from 'axios';


async function searchBatchVideos(ids) {
  
    
    // Construct the base API URL with required parameters
    let apiUrl = `https://ytapi.scrappa.co/videos/batch?ids=${encodeURIComponent(ids)}`;


    try {
        console.log(`Fetching from: ${apiUrl}`);
        const response = await axios.get(apiUrl);
        const data = response.data.videos;
        
        // Save the fetched data to the default dataset.
        await Actor.pushData(data);
        console.log(`Successfully fetched ${data.length} batch videos for query: ${ids}`);
        
       
    } catch (error) {

        console.error(`Failed to fetch batch videos for query: ${ids}`, error.message);
        throw error;
    }
}

// Main Actor logic
Actor.main(async () => {
    // The init() call configures the Actor for its environment.
    await Actor.init();

    const input = await Actor.getInput();
    const { ids } = input;

    // Directly call the function with the input, as there is only one possible task.
    await searchBatchVideos(ids);

    // Gracefully exit the Actor process.
    await Actor.exit();
});
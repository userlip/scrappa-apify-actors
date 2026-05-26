#!/usr/bin/env node

import path from 'node:path';
import { pathToFileURL } from 'node:url';
import {
  buildLocalActorIndex,
  fetchLiveActors,
  findMissingLiveActors,
  getToken,
} from './live-actor-source-utils.mjs';

if (isCliEntrypoint()) {
  runCli().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(2);
  });
}

async function runCli() {
  const args = parseArgs(process.argv.slice(2));

  if (args.help) {
    printHelp();
    return;
  }

  const token = getToken();
  const actorsRoot = path.join(process.cwd(), 'actors');
  const liveActors = await fetchLiveActors(token);
  const localActors = await buildLocalActorIndex(actorsRoot);
  const localActorSourceCount = new Set([...localActors.values()].map((actor) => actor.directory)).size;
  const missingActors = findMissingLiveActors(liveActors, localActors);

  if (args.json) {
    console.log(JSON.stringify({
      liveActors: liveActors.length,
      localActorSources: localActorSourceCount,
      missingActors: missingActors.map((actor) => ({
        actorId: actor.id,
        slug: actor.name,
        title: actor.title || null,
      })),
    }, null, 2));
  } else {
    console.log(`Live Apify actors: ${liveActors.length}`);
    console.log(`Local actor sources: ${localActorSourceCount}`);
    console.log(`Missing local sources: ${missingActors.length}`);

    for (const actor of missingActors) {
      console.log(`- ${actor.id} - ${actor.name}`);
    }
  }

  if (missingActors.length > 0) {
    process.exit(1);
  }
}

function parseArgs(argv) {
  const parsed = {
    help: false,
    json: false,
  };

  for (const arg of argv) {
    if (arg === '--help' || arg === '-h') {
      parsed.help = true;
    } else if (arg === '--json') {
      parsed.json = true;
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }

  return parsed;
}

function isCliEntrypoint() {
  return process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
}

function printHelp() {
  console.log(`Verify every live Apify actor has a local actors/{slug} source directory.

Usage:
  APIFY_TOKEN=... pnpm audit:live-actors
  APIFY_TOKEN=... pnpm audit:live-actors --json

Alias:
  APIFY_TOKEN=... pnpm verify:live-actors
`);
}

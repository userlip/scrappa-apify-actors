#!/usr/bin/env node

import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import {
  buildLocalActorIndex,
  fetchActorDetail,
  fetchLiveActors,
  getToken,
  isSafeSourcePath,
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
  const repoRoot = process.cwd();
  const actorsRoot = path.join(repoRoot, 'actors');
  const localActors = await buildLocalActorIndex(actorsRoot);
  const liveActors = await fetchLiveActors(token);
  const requestedSlugs = args.slugs.length > 0 ? new Set(args.slugs) : null;
  const missingActors = liveActors
    .filter((actor) => !requestedSlugs || requestedSlugs.has(actor.name))
    .filter((actor) => !localActors.has(actor.name))
    .sort((left, right) => left.name.localeCompare(right.name));

  const unknownRequestedSlugs = requestedSlugs
    ? [...requestedSlugs].filter((slug) => !liveActors.some((actor) => actor.name === slug))
    : [];
  if (unknownRequestedSlugs.length > 0) {
    throw new Error(`Requested slugs are not present in the live actor list: ${unknownRequestedSlugs.join(', ')}`);
  }

  if (missingActors.length === 0) {
    console.log('No missing live actor sources to import.');
    return;
  }

  const imported = [];
  for (const actor of missingActors) {
    const detail = await fetchActorDetail(actor.id, token);
    const version = detail.versions?.[0];
    const sourceFiles = version?.sourceFiles;
    if (!Array.isArray(sourceFiles) || sourceFiles.length === 0) {
      throw new Error(`${actor.id} - ${actor.name} has no sourceFiles on versions[0].`);
    }

    const actorDir = path.join(actorsRoot, actor.name);
    const filesWritten = await writeSourceFiles(actorDir, sourceFiles);
    imported.push({
      actorId: actor.id,
      slug: actor.name,
      versionNumber: version.versionNumber || null,
      filesWritten,
    });
  }

  console.log(JSON.stringify({ imported }, null, 2));
}

async function writeSourceFiles(actorDir, sourceFiles) {
  let filesWritten = 0;

  await mkdir(actorDir, { recursive: false });

  for (const sourceFile of sourceFiles) {
    const fileName = sourceFile.name;
    if (!isSafeSourcePath(fileName)) {
      throw new Error(`Unsafe source file path from Apify: ${fileName}`);
    }

    const targetPath = path.join(actorDir, fileName);
    if (typeof sourceFile.content !== 'string') {
      await mkdir(targetPath, { recursive: true });
      continue;
    }

    await mkdir(path.dirname(targetPath), { recursive: true });
    const content = sourceFile.format === 'BASE64'
      ? Buffer.from(sourceFile.content, 'base64')
      : sourceFile.content;
    await writeFile(targetPath, content);
    filesWritten += 1;
  }

  return filesWritten;
}

function parseArgs(argv) {
  const parsed = {
    help: false,
    slugs: [],
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--help' || arg === '-h') {
      parsed.help = true;
    } else if (arg === '--slug') {
      const value = argv[index + 1];
      if (!value || value.startsWith('--')) {
        throw new Error('--slug requires an actor slug.');
      }

      parsed.slugs.push(value);
      index += 1;
    } else if (arg.startsWith('--slug=')) {
      parsed.slugs.push(arg.slice('--slug='.length));
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
  console.log(`Import missing live Apify SOURCE_FILES actors into actors/{slug}.

Usage:
  APIFY_TOKEN=... pnpm import:live-actors
  APIFY_TOKEN=... pnpm import:live-actors --slug youtube-api-video-chapters

The importer creates only missing actor directories. Existing local actor
directories are never overwritten by this command.
`);
}

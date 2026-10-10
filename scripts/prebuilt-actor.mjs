// Compiles a Rust Actor locally and returns Apify SOURCE_FILES that only copy the
// finished binary. Apify builders run with 4 GB memory, so compiling there costs
// ~0.15 CU per build; copying a prebuilt binary costs ~0.003 CU.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import zlib from 'node:zlib';

// Same toolchain and glibc as the Actor Dockerfiles, so the binary runs on bookworm-slim.
const BUILDER_IMAGE = 'rust:1.90-slim-bookworm';
// Apify rejects versions whose source files exceed 3 MB in total.
const MAX_SOURCE_BYTES = 3 * 1024 * 1024;
const CACHE_DIR = process.env.PREBUILT_CACHE_DIR || path.join(os.homedir(), '.cache', 'scrappa-apify-build');
const SIZE_PROFILE = {
  CARGO_PROFILE_RELEASE_STRIP: 'true',
  CARGO_PROFILE_RELEASE_OPT_LEVEL: 's',
  CARGO_PROFILE_RELEASE_LTO: 'true',
  CARGO_PROFILE_RELEASE_CODEGEN_UNITS: '1',
};
const METADATA_SKIP = new Set(['Dockerfile', '.dockerignore', 'actor.gz']);

export const RUNTIME_DOCKERFILE = `FROM debian:bookworm-slim
COPY .actor/actor.gz /tmp/actor.gz
RUN gunzip -c /tmp/actor.gz > /usr/local/bin/actor \\
    && rm /tmp/actor.gz \\
    && chmod 755 /usr/local/bin/actor \\
    && useradd --create-home --uid 1000 myuser
USER myuser
WORKDIR /home/myuser
CMD ["/usr/local/bin/actor"]
`;

export function binaryName(dir) {
  const dockerfiles = [path.join(dir, '.actor', 'Dockerfile'), path.join(dir, 'Dockerfile')];
  for (const file of dockerfiles) {
    if (!fs.existsSync(file)) continue;
    const match = fs.readFileSync(file, 'utf8').match(/target\/release\/([A-Za-z0-9_-]+)/);
    if (match) return match[1];
  }
  throw new Error(`${dir}: no target/release/<binary> in Dockerfile`);
}

export function compileBinary(dir) {
  const absolute = path.resolve(dir);
  const target = path.join(CACHE_DIR, 'target');
  const cargoHome = path.join(CACHE_DIR, 'cargo');
  fs.mkdirSync(target, { recursive: true });
  fs.mkdirSync(cargoHome, { recursive: true });
  const env = Object.entries(SIZE_PROFILE).flatMap(([key, value]) => ['-e', `${key}=${value}`]);
  execFileSync('docker', [
    'run', '--rm',
    '-u', `${process.getuid()}:${process.getgid()}`,
    '-v', `${absolute}:/src:ro`, '-v', `${target}:/target`, '-v', `${cargoHome}:/cargo`,
    '-e', 'CARGO_HOME=/cargo', '-e', 'CARGO_TARGET_DIR=/target', ...env,
    '-w', '/src', BUILDER_IMAGE,
    'cargo', 'build', '--release', '--locked', '--quiet',
  ], { stdio: ['ignore', 'inherit', 'inherit'] });
  const binary = path.join(target, 'release', binaryName(dir));
  if (!fs.existsSync(binary)) throw new Error(`${dir}: compiled binary ${binary} missing`);
  return fs.readFileSync(binary);
}

function metadataFiles(dir) {
  const files = [];
  const actorDir = path.join(dir, '.actor');
  for (const entry of fs.readdirSync(actorDir, { withFileTypes: true })) {
    if (!entry.isFile() || METADATA_SKIP.has(entry.name)) continue;
    let content = fs.readFileSync(path.join(actorDir, entry.name), 'utf8');
    if (entry.name === 'actor.json') {
      const actorJson = JSON.parse(content);
      actorJson.dockerfile = './Dockerfile';
      content = `${JSON.stringify(actorJson, null, 2)}\n`;
    }
    files.push({ name: `.actor/${entry.name}`, format: 'TEXT', content });
  }
  const rootReadme = path.join(dir, 'README.md');
  if (fs.existsSync(rootReadme)) files.push({ name: 'README.md', format: 'TEXT', content: fs.readFileSync(rootReadme, 'utf8') });
  return files;
}

export function prebuiltSourceFiles(dir, binary = compileBinary(dir)) {
  const files = [
    ...metadataFiles(dir),
    { name: '.actor/Dockerfile', format: 'TEXT', content: RUNTIME_DOCKERFILE },
    { name: '.actor/actor.gz', format: 'BASE64', content: zlib.gzipSync(binary, { level: 9 }).toString('base64') },
  ].sort((a, b) => a.name.localeCompare(b.name));
  const size = files.reduce((sum, file) => sum + Buffer.byteLength(file.content), 0);
  if (size >= MAX_SOURCE_BYTES) throw new Error(`${dir}: prebuilt source files are ${size} bytes, Apify limit is ${MAX_SOURCE_BYTES}`);
  return files;
}

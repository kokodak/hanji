import { readdir } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { pathToFileURL } from 'node:url';
import { createServer } from 'vite';

async function findTestFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);

    if (entry.isDirectory()) {
      files.push(...(await findTestFiles(entryPath)));
      continue;
    }

    if (entry.name.endsWith('.test.ts')) {
      files.push(entryPath);
    }
  }

  return files.sort();
}

const root = process.cwd();
const server = await createServer({
  configFile: false,
  root,
  logLevel: 'silent',
  server: { middlewareMode: true }
});

let passed = 0;
let failed = 0;

try {
  const testFiles = await findTestFiles(path.join(root, 'src'));

  if (testFiles.length === 0) {
    console.log('No test files found.');
    process.exitCode = 1;
  }

  for (const testFile of testFiles) {
    const modulePath = pathToFileURL(testFile).href;
    const module = await server.ssrLoadModule(modulePath);
    const tests = module.tests ?? [];

    for (const test of tests) {
      try {
        await test.run();
        passed += 1;
        console.log(`ok ${test.name}`);
      } catch (error) {
        failed += 1;
        console.error(`not ok ${test.name}`);
        console.error(error);
      }
    }
  }
} finally {
  await server.close();
}

console.log(`${passed} passed, ${failed} failed`);

if (failed > 0) {
  process.exitCode = 1;
}

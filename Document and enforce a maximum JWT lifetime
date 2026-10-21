#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import openapiTS, { astToString } from "openapi-typescript";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryDirectory = resolve(scriptDirectory, "../..");
const outputPath = resolve(
  process.cwd(),
  process.env.OUT ?? resolve(scriptDirectory, "../types/api.d.ts"),
);

let spec;
if (process.env.SPEC) {
  const specPath = resolve(process.cwd(), process.env.SPEC);
  const source = await readFile(specPath, "utf8");
  spec = JSON.parse(source.replace(/^\uFEFF/, ""));
} else {
  const result = spawnSync(
    "cargo",
    [
      "run",
      "--quiet",
      "--manifest-path",
      resolve(repositoryDirectory, "backend/Cargo.toml"),
      "--bin",
      "predifi-openapi",
    ],
    { cwd: repositoryDirectory, encoding: "utf8" },
  );

  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    process.stderr.write(result.stderr);
    process.exit(result.status ?? 1);
  }

  spec = JSON.parse(result.stdout);
}

const types = await openapiTS(spec);
await mkdir(dirname(outputPath), { recursive: true });
await writeFile(outputPath, astToString(types));
console.error(`Generated API types at ${outputPath}`);

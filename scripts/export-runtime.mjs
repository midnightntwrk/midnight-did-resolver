// This file is part of midnightntwrk/midnight-did-resolver.
// Copyright (C) Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
import { cpSync, existsSync, lstatSync, mkdirSync, readdirSync, readFileSync, realpathSync, rmSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const services = new Set(["did-resolver-service", "did-manager-service"]);
const workspaceDirs = ["did-resolver-service", "did-manager-service", "secret-storage", "docs-site"];
const readJson = (file) => JSON.parse(readFileSync(file, "utf8"));

// Validate every link, including nested production dependency and native-loader links.
export function inspectRuntime(root) {
  root = realpathSync(root);
  let files = 0;
  let links = 0;
  function walk(dir) {
    for (const name of readdirSync(dir)) {
      const file = path.join(dir, name);
      const stat = lstatSync(file);
      if (stat.isSymbolicLink()) {
        const target = realpathSync(file); // Throws for dangling links.
        if (target !== root && !target.startsWith(root + path.sep)) {
          throw new Error("Runtime link escapes export: " + file);
        }
        links++;
      } else if (stat.isDirectory()) walk(file);
      else files++;
    }
  }
  walk(root);
  return { files, links };
}

// Input is the built, globally production-pruned workspace. This deliberately
// retains a production dependency superset, not a service-minimal npm graph.
export function exportRuntime(source, output, service) {
  if (!services.has(service)) throw new Error("Unknown service: " + service);
  source = realpathSync(source);
  output = path.resolve(output);
  if (existsSync(output)) throw new Error("Output must not exist: " + output);
  const selected = service === "did-manager-service" ? [service, "secret-storage"] : [service];
  for (const dir of selected) {
    if (!existsSync(path.join(source, dir, "dist", "index.js"))) throw new Error("Missing built entrypoint: " + dir);
  }
  mkdirSync(output, { recursive: true });
  const copy = (relative) => cpSync(path.join(source, relative), path.join(output, relative), {
    recursive: true, verbatimSymlinks: true,
  });
  for (const file of ["package.json", "version", "node_modules"]) copy(file);
  // npm installation bookkeeping is not part of the runtime payload.
  rmSync(path.join(output, "node_modules", ".package-lock.json"), { force: true });
  for (const dir of selected) {
    mkdirSync(path.join(output, dir), { recursive: true });
    copy(dir + "/package.json");
    copy(dir + "/dist");
    // Accommodate npm placing production dependencies locally in a workspace.
    if (existsSync(path.join(source, dir, "node_modules"))) copy(dir + "/node_modules");
    function strip(dir) {
      for (const name of readdirSync(dir)) {
        const file = path.join(dir, name);
        const stat = lstatSync(file);
        if (stat.isDirectory()) strip(file);
        else if (/\.map$|\.d\.[cm]?ts$|\.tsbuildinfo$/.test(name)) rmSync(file);
      }
    }
    strip(path.join(output, dir, "dist"));
  }
  // Only remove known omitted workspace links. Vendor package internals,
  // production .bin links, optional dependencies, licenses and assets stay intact.
  for (const dir of workspaceDirs.filter((dir) => !selected.includes(dir))) {
    const name = readJson(path.join(source, dir, "package.json")).name;
    const link = path.join(output, "node_modules", name);
    try {
      if (!lstatSync(link).isSymbolicLink()) throw new Error("Expected workspace link: " + name);
      rmSync(link);
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
  }
  return inspectRuntime(output);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [command, ...args] = process.argv.slice(2);
  if (command === "export" && args.length === 3) {
    console.log(JSON.stringify(exportRuntime(...args)));
  } else if (command === "inspect" && args.length === 1) {
    console.log(JSON.stringify(inspectRuntime(args[0])));
  } else {
    throw new Error("Usage: node export-runtime.mjs export SOURCE OUTPUT SERVICE | inspect ROOT");
  }
}

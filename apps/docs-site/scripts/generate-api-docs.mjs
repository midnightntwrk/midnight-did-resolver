import { existsSync, mkdirSync, readdirSync, renameSync, rmSync, statSync } from "node:fs";
import { readFile, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { execFileSync } from "node:child_process";

const repoRoot = resolve(new URL("../../..", import.meta.url).pathname);
const docsRoot = resolve(repoRoot, "apps", "docs-site");
const outputRoot = resolve(docsRoot, "api", "reference");

const packages = [
  {
    slug: "secret-storage",
    displayName: "@midnight-ntwrk/midnight-did-secret-storage",
    entryPoint: resolve(repoRoot, "packages", "secret-storage", "src", "index.ts"),
    tsconfig: resolve(repoRoot, "packages", "secret-storage", "tsconfig.build.json"),
  },
];

const rewriteReadmeLinks = async (rootDir) => {
  for (const entry of readdirSync(rootDir)) {
    const target = join(rootDir, entry);
    if (statSync(target).isDirectory()) {
      await rewriteReadmeLinks(target);
      continue;
    }

    if (!target.endsWith(".md")) continue;

    const content = await readFile(target, "utf8");
    const next = content
      .replaceAll("../README.md", "../index.md")
      .replaceAll("./README.md", "./index.md")
      .replaceAll("/README.md", "/index.md");

    if (next !== content) {
      await writeFile(target, next, "utf8");
    }
  }
};

mkdirSync(outputRoot, { recursive: true });

execFileSync(process.platform === "win32" ? "pnpm.cmd" : "pnpm", ["--filter", "@midnight-ntwrk/midnight-did-secret-storage", "build"], {
  cwd: repoRoot,
  stdio: "inherit",
  env: process.env,
});

for (const entry of readdirSync(outputRoot)) {
  const target = join(outputRoot, entry);
  if (statSync(target).isDirectory()) {
    rmSync(target, { recursive: true, force: true });
  }
}

for (const pkg of packages) {
  const outDir = resolve(outputRoot, pkg.slug);
  mkdirSync(outDir, { recursive: true });

  execFileSync(
    process.platform === "win32" ? "npx.cmd" : "npx",
    [
      "typedoc",
      "--plugin",
      "typedoc-plugin-markdown",
      "--tsconfig",
      pkg.tsconfig,
      "--entryPoints",
      pkg.entryPoint,
      "--out",
      outDir,
      "--readme",
      "none",
      "--hideGenerator",
      "--disableSources",
      "--excludeExternals",
      "--exclude",
      "**/*.test.ts",
      "--exclude",
      "**/test/**",
    ],
    { cwd: docsRoot, stdio: "inherit", env: process.env },
  );

  const readmePath = join(outDir, "README.md");
  const indexPath = join(outDir, "index.md");
  if (existsSync(readmePath)) renameSync(readmePath, indexPath);

  if (existsSync(indexPath)) {
    const content = `# ${pkg.displayName}\n\nGenerated API reference.\n\n`;
    const current = await readFile(indexPath, "utf8");
    await writeFile(indexPath, `${content}${current}`, "utf8");
  }

  await rewriteReadmeLinks(outDir);
}

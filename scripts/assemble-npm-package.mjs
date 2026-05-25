import { copyFile, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";

const packageRoot = path.resolve(process.argv[2] ?? "pkg/npm");
const repoRoot = process.cwd();

const targets = ["bundler", "nodejs", "web"];

async function readPackageJson(target) {
  const packageJsonPath = path.join(packageRoot, target, "package.json");
  return JSON.parse(await readFile(packageJsonPath, "utf8"));
}

for (const target of targets) {
  await readPackageJson(target);
  for (const generatedMetadata of [".gitignore", "LICENSE.md", "README.md"]) {
    await rm(path.join(packageRoot, target, generatedMetadata), { force: true });
  }
}

const bundlerPackage = await readPackageJson("bundler");

const packageJson = {
  name: bundlerPackage.name,
  version: bundlerPackage.version,
  description: bundlerPackage.description,
  license: bundlerPackage.license,
  repository: bundlerPackage.repository,
  collaborators: bundlerPackage.collaborators,
  keywords: bundlerPackage.keywords,
  files: [
    "bundler",
    "nodejs",
    "web",
    "LICENSE.md",
    "README.md",
  ],
  main: "./nodejs/sudoku2.js",
  module: "./bundler/sudoku2.js",
  types: "./bundler/sudoku2.d.ts",
  exports: {
    ".": {
      types: "./bundler/sudoku2.d.ts",
      node: {
        types: "./nodejs/sudoku2.d.ts",
        import: "./nodejs/sudoku2.js",
        require: "./nodejs/sudoku2.js",
        default: "./nodejs/sudoku2.js",
      },
      import: "./bundler/sudoku2.js",
      default: "./bundler/sudoku2.js",
    },
    "./bundler": {
      types: "./bundler/sudoku2.d.ts",
      import: "./bundler/sudoku2.js",
      default: "./bundler/sudoku2.js",
    },
    "./nodejs": {
      types: "./nodejs/sudoku2.d.ts",
      require: "./nodejs/sudoku2.js",
      default: "./nodejs/sudoku2.js",
    },
    "./web": {
      types: "./web/sudoku2.d.ts",
      import: "./web/sudoku2.js",
      default: "./web/sudoku2.js",
    },
    "./package.json": "./package.json",
  },
  sideEffects: [
    "./bundler/sudoku2.js",
    "./web/sudoku2.js",
    "./*/snippets/*",
  ],
};

await writeFile(
  path.join(packageRoot, "package.json"),
  `${JSON.stringify(packageJson, null, 2)}\n`,
);
await copyFile(path.join(repoRoot, "LICENSE.md"), path.join(packageRoot, "LICENSE.md"));
await copyFile(path.join(repoRoot, "README.md"), path.join(packageRoot, "README.md"));

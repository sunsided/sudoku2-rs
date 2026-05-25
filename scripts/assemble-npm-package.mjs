import { copyFile, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";

const packageRoot = path.resolve(process.argv[2] ?? "pkg/npm");
const repoRoot = process.cwd();

const targets = ["bundler", "nodejs", "web"];
const targetPackages = new Map();

async function readPackageJson(target) {
  const packageJsonPath = path.join(packageRoot, target, "package.json");
  return JSON.parse(await readFile(packageJsonPath, "utf8"));
}

function targetEntry(target, field) {
  const packageJson = targetPackages.get(target);
  const entry = packageJson?.[field];

  if (typeof entry !== "string" || entry.length === 0) {
    throw new Error(`${target} package.json does not define ${field}`);
  }

  return `./${target}/${entry.replace(/^\.\//, "")}`;
}

for (const target of targets) {
  targetPackages.set(target, await readPackageJson(target));
  for (const generatedMetadata of [".gitignore", "LICENSE.md", "README.md"]) {
    await rm(path.join(packageRoot, target, generatedMetadata), { force: true });
  }
}

const bundlerPackage = targetPackages.get("bundler");

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
  main: targetEntry("nodejs", "main"),
  module: targetEntry("bundler", "main"),
  types: targetEntry("bundler", "types"),
  exports: {
    ".": {
      types: targetEntry("bundler", "types"),
      node: {
        types: targetEntry("nodejs", "types"),
        import: targetEntry("nodejs", "main"),
        require: targetEntry("nodejs", "main"),
        default: targetEntry("nodejs", "main"),
      },
      import: targetEntry("bundler", "main"),
      default: targetEntry("bundler", "main"),
    },
    "./bundler": {
      types: targetEntry("bundler", "types"),
      import: targetEntry("bundler", "main"),
      default: targetEntry("bundler", "main"),
    },
    "./nodejs": {
      types: targetEntry("nodejs", "types"),
      require: targetEntry("nodejs", "main"),
      default: targetEntry("nodejs", "main"),
    },
    "./web": {
      types: targetEntry("web", "types"),
      import: targetEntry("web", "main"),
      default: targetEntry("web", "main"),
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

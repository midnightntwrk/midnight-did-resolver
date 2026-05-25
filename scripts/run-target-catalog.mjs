#!/usr/bin/env node
import { stdout } from "node:process";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const laneTargets = [
  {
    name: "secret-storage",
    label: "Secret storage pipeline",
    command: "./scripts/run-secret-storage.sh",
    description: "Secret-storage package lint, build, and unit-test lane.",
    supportsLight: true,
    supportsStrict: true,
    supportsMetrics: true,
  },
  {
    name: "resolver",
    label: "TypeScript resolver service pipeline",
    command: "./scripts/run-resolver.sh",
    description: "Node resolver service build, unit tests, and optional integration lane.",
    supportsLight: true,
    supportsStrict: true,
    supportsMetrics: true,
  },
  {
    name: "manager",
    label: "DID manager service pipeline",
    command: "./scripts/run-manager.sh",
    description: "Manager service build, unit tests, and optional browser E2E lane.",
    supportsLight: true,
    supportsStrict: true,
    supportsMetrics: true,
  },
  {
    name: "docs",
    label: "Docs pipeline",
    command: "./scripts/run-docs.sh",
    description: "Resolver documentation site build lane.",
    supportsLight: false,
    supportsStrict: false,
    supportsMetrics: true,
  },
];

export const laneTargetByName = new Map(laneTargets.map((target) => [target.name, target]));
export const fullPipelineOrder = ["secret-storage", "resolver", "manager", "docs"];
export const pipelineSteps = fullPipelineOrder.map((name) => {
  const laneTarget = laneTargetByName.get(name);
  if (!laneTarget) throw new Error(`Missing full pipeline lane target: ${name}`);
  return laneTarget;
});

export const targets = [
  {
    name: "full",
    description: "Run the full TypeScript workspace validation pipeline. This is the default target.",
    supportsLight: true,
    supportsStrict: true,
    supportsMetrics: true,
  },
  ...laneTargets.map(({ name, description, supportsLight, supportsStrict, supportsMetrics }) => ({
    name,
    description,
    supportsLight,
    supportsStrict,
    supportsMetrics,
  })),
  {
    name: "targets",
    description: "Print this runner target catalog.",
    supportsLight: false,
    supportsStrict: false,
    supportsMetrics: false,
  },
  {
    name: "help",
    description: "Print runner usage and target details.",
    supportsLight: false,
    supportsStrict: false,
    supportsMetrics: false,
  },
];

export const targetNames = new Set(targets.map((target) => target.name));
export const targetByName = new Map(targets.map((target) => [target.name, target]));

export const stepsForTarget = (targetName = "full") => {
  if (targetName === "full") return pipelineSteps;
  const laneTarget = laneTargetByName.get(targetName);
  if (!laneTarget) throw new Error(`Unknown executable target: ${targetName}`);
  return [laneTarget];
};

const printRows = (rows) => {
  const width = Math.max(...rows.map(([name]) => name.length));
  for (const [name, description] of rows) stdout.write(`  ${name.padEnd(width)}  ${description}\n`);
};

export const printTargets = () => {
  stdout.write("Targets:\n");
  printRows(targets.map((target) => [target.name, target.description]));
  stdout.write("\nPipeline steps for target 'full':\n");
  printRows(pipelineSteps.map((step) => [step.command, `${step.label}: ${step.description}`]));
  stdout.write("\nSingle-lane target commands:\n");
  printRows(laneTargets.map((target) => [target.name, target.command]));
};

const isDirectExecution = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
const [command, value] = process.argv.slice(2);

if (isDirectExecution) {
  switch (command) {
    case "--json":
      stdout.write(`${JSON.stringify({ targets, pipelineSteps }, null, 2)}\n`);
      break;
    case "--names":
      stdout.write(`${targets.map((target) => target.name).join("\n")}\n`);
      break;
    case "--has-target":
      process.exit(targetNames.has(value) ? 0 : 1);
      break;
    case "--supports-light":
      process.exit(targetByName.get(value)?.supportsLight ? 0 : 1);
      break;
    case "--supports-strict":
      process.exit(targetByName.get(value)?.supportsStrict ? 0 : 1);
      break;
    case "--step-labels":
      stdout.write(`${stepsForTarget(value).map((step) => step.label).join("\n")}\n`);
      break;
    case "--step-commands":
      stdout.write(`${stepsForTarget(value).map((step) => step.command).join("\n")}\n`);
      break;
    case "--targets":
    case "--help":
    case undefined:
      printTargets();
      break;
    default:
      console.error(`Unknown run target catalog command: ${command}`);
      process.exit(1);
  }
}

import init, { generate_puzzle, solve_puzzle } from "../pkg/sudoku2.js";

const byId = (id) => document.getElementById(id);

function setOutput(id, data) {
  byId(id).textContent = typeof data === "string" ? data : JSON.stringify(data, null, 2);
}

function asOptionalText(value) {
  const trimmed = value.trim();
  return trimmed.length === 0 ? undefined : trimmed;
}

let lastGenerated = undefined;
let lastGeneratedVariant = undefined;

await init();

byId("solve-button").addEventListener("click", () => {
  try {
    const request = {
      puzzle: byId("solve-puzzle").value,
      variant: byId("solve-variant").value,
      format: byId("solve-format").value,
      region_line: asOptionalText(byId("solve-region").value),
    };
    setOutput("solve-output", solve_puzzle(request));
  } catch (error) {
    setOutput("solve-output", String(error));
  }
});

byId("generate-button").addEventListener("click", () => {
  try {
    const seedText = asOptionalText(byId("gen-seed").value);
    const request = {
      variant: byId("gen-variant").value,
      target_difficulty: byId("gen-difficulty").value,
      symmetry: byId("gen-symmetry").value,
      max_attempts: Number(byId("gen-attempts").value),
      seed: seedText === undefined ? undefined : Number(seedText),
    };
    const response = generate_puzzle(request);
    lastGenerated = response;
    lastGeneratedVariant = request.variant;
    setOutput("generate-output", response);
  } catch (error) {
    lastGenerated = undefined;
    lastGeneratedVariant = undefined;
    setOutput("generate-output", String(error));
  }
});

byId("send-to-solver-button").addEventListener("click", () => {
  if (lastGenerated === undefined || lastGeneratedVariant === undefined) {
    setOutput("generate-output", "Generate a puzzle first.");
    return;
  }

  byId("solve-variant").value = lastGeneratedVariant;
  byId("solve-format").value = "line";
  byId("solve-puzzle").value = lastGenerated.puzzle_line ?? "";
  byId("solve-region").value = lastGenerated.region_line ?? "";
  setOutput("solve-output", "Loaded generated puzzle into solver input.");
});

byId("solve-clear-button").addEventListener("click", () => {
  byId("solve-variant").value = "standard";
  byId("solve-format").value = "auto";
  byId("solve-puzzle").value = "";
  byId("solve-region").value = "";
  setOutput("solve-output", "");
});

import init, { generate_puzzle, solve_puzzle } from "../pkg/sudoku2.js";

const byId = (id) => document.getElementById(id);

function setOutput(id, data) {
  byId(id).textContent = typeof data === "string" ? data : JSON.stringify(data, null, 2);
}

function asOptionalText(value) {
  const trimmed = value.trim();
  return trimmed.length === 0 ? undefined : trimmed;
}

let lastGeneratedPuzzle = undefined;

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
    lastGeneratedPuzzle = { response, variant: request.variant };
    setOutput("generate-output", response);
  } catch (error) {
    lastGeneratedPuzzle = undefined;
    setOutput("generate-output", String(error));
  }
});

byId("send-to-solver-button").addEventListener("click", () => {
  if (lastGeneratedPuzzle === undefined) {
    setOutput("solve-output", "No puzzle available. Please generate a puzzle first using the Generator section above.");
    return;
  }

  byId("solve-variant").value = lastGeneratedPuzzle.variant;
  byId("solve-format").value = "line";
  byId("solve-puzzle").value = lastGeneratedPuzzle.response.puzzle_line ?? "";
  byId("solve-region").value = lastGeneratedPuzzle.response.region_line ?? "";
  setOutput("solve-output", "Loaded generated puzzle into solver input.");
});

byId("solve-clear-button").addEventListener("click", () => {
  byId("solve-variant").value = "standard";
  byId("solve-format").value = "auto";
  byId("solve-puzzle").value = "";
  byId("solve-region").value = "";
  setOutput("solve-output", "");
});

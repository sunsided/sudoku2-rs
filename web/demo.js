import init, { generate_puzzle, solve_puzzle } from "../pkg/sudoku2.js";

const byId = (id) => document.getElementById(id);

function setOutput(id, data) {
  byId(id).textContent = typeof data === "string" ? data : JSON.stringify(data, null, 2);
}

function asOptionalText(value) {
  const trimmed = value.trim();
  return trimmed.length === 0 ? undefined : trimmed;
}

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
    setOutput("generate-output", generate_puzzle(request));
  } catch (error) {
    setOutput("generate-output", String(error));
  }
});

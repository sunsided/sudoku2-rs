import init, { solve_puzzle, solve_step } from "../pkg/sudoku2.js";

const byId = (id) => document.getElementById(id);

function setOutput(id, data) {
  byId(id).textContent = typeof data === "string" ? data : JSON.stringify(data, null, 2);
}

function setStatus(id, text) {
  byId(id).textContent = text ?? "";
}

function setGenerationProgress(value, max, hidden = false) {
  const progress = byId("generate-progress");
  progress.value = value;
  progress.max = Math.max(max, 1);
  progress.hidden = hidden;
}

function asOptionalText(value) {
  const trimmed = value.trim();
  return trimmed.length === 0 ? undefined : trimmed;
}

function renderBoard(id, line, regionLine, highlight) {
  const board = byId(id);
  board.textContent = "";
  const normalized = (line ?? "").replace(/[^0-9.]/g, "");
  const regions = (regionLine ?? "").replace(/[^A-I]/g, "");

  for (let index = 0; index < 81; index += 1) {
    const cell = document.createElement("div");
    const value = normalized[index] ?? ".";
    cell.className = "sudoku-cell";
    if (value === "." || value === "0") {
      cell.classList.add("empty");
      cell.textContent = "";
    } else {
      cell.textContent = value;
    }

    const region = regions[index];
    if (region !== undefined && /[A-I]/.test(region)) {
      cell.classList.add(`region-${region}`);
    }

    if (highlight?.index === index) {
      cell.classList.add("highlight");
    }

    board.appendChild(cell);
  }
}

function solveRequestFromForm() {
  return {
    puzzle: byId("solve-puzzle").value,
    variant: byId("solve-variant").value,
    format: byId("solve-format").value,
    region_line: asOptionalText(byId("solve-region").value),
  };
}

function generateRequestFromForm() {
  const seedText = asOptionalText(byId("gen-seed").value);
  return {
    variant: byId("gen-variant").value,
    target_difficulty: byId("gen-difficulty").value,
    symmetry: byId("gen-symmetry").value,
    max_attempts: Number(byId("gen-attempts").value),
    seed: seedText === undefined ? undefined : Number(seedText),
  };
}

let lastGeneratedPuzzle = undefined;
let generationWorker = undefined;
let solverBaselineLine = undefined;
let solverProgressLine = undefined;
let solverRegionLine = undefined;

function resetSolverProgress(line, regionLine) {
  solverBaselineLine = line;
  solverProgressLine = line;
  solverRegionLine = regionLine;
}

function currentStepRequest() {
  const request = solveRequestFromForm();
  if (solverProgressLine !== undefined) {
    request.puzzle = solverProgressLine;
    request.format = "line";
    request.region_line = solverRegionLine;
  } else {
    resetSolverProgress(request.puzzle, request.region_line);
  }
  return request;
}

await init();
renderBoard("solve-board");
renderBoard("solve-solution-board");
renderBoard("generate-board");
renderBoard("generate-solution-board");

for (const id of ["solve-puzzle", "solve-region", "solve-variant", "solve-format"]) {
  byId(id).addEventListener("input", () => resetSolverProgress(undefined, undefined));
  byId(id).addEventListener("change", () => resetSolverProgress(undefined, undefined));
}

byId("solve-button").addEventListener("click", () => {
  try {
    const request = solveRequestFromForm();
    resetSolverProgress(request.puzzle, request.region_line);
    const response = solve_puzzle(request);
    setOutput("solve-output", response);
    setStatus("solve-status", response.error ?? (response.solved ? "Solved" : "No solution found"));
    solverProgressLine = response.state_line;
    solverRegionLine = response.region_line;
    renderBoard("solve-board", solverBaselineLine ?? byId("solve-puzzle").value, response.region_line);
    renderBoard("solve-solution-board", response.state_line, response.region_line);
  } catch (error) {
    setOutput("solve-output", String(error));
    setStatus("solve-status", String(error));
  }
});

byId("solve-step-button").addEventListener("click", () => {
  try {
    const response = solve_step(currentStepRequest());
    setOutput("solve-output", response);
    setStatus("solve-status", response.error ?? response.explanation ?? "No step available");
    if (response.state_line !== undefined) {
      solverProgressLine = response.state_line;
      solverRegionLine = response.region_line;
    }
    renderBoard("solve-board", solverBaselineLine, solverRegionLine, response.cell);
    renderBoard("solve-solution-board", solverProgressLine, solverRegionLine, response.cell);
  } catch (error) {
    setOutput("solve-output", String(error));
    setStatus("solve-status", String(error));
  }
});

byId("generate-button").addEventListener("click", () => {
  if (generationWorker !== undefined) {
    generationWorker.terminate();
  }

  const request = generateRequestFromForm();
  generationWorker = new Worker(new URL("./generator-worker.js", import.meta.url), { type: "module" });
  lastGeneratedPuzzle = undefined;
  byId("generate-button").disabled = true;
  renderBoard("generate-board");
  renderBoard("generate-solution-board");
  setGenerationProgress(0, request.max_attempts);
  setStatus("generate-status", `Starting generation (0/${request.max_attempts})...`);
  setOutput("generate-output", "");

  generationWorker.addEventListener("message", (event) => {
    const message = event.data;
    if (message.type === "progress") {
      const progress = message.progress;
      setGenerationProgress(progress.attempt, progress.max_attempts);
      setStatus(
        "generate-status",
        `${progress.event.replaceAll("_", " ")} (${progress.attempt}/${progress.max_attempts})`
      );
      if (progress.puzzle_line !== undefined) {
        renderBoard("generate-board", progress.puzzle_line, progress.region_line);
        renderBoard("generate-solution-board", progress.solution_line, progress.region_line);
      }
      setOutput("generate-output", progress);
      return;
    }

    byId("generate-button").disabled = false;
    generationWorker?.terminate();
    generationWorker = undefined;

    if (message.type === "complete") {
      const response = message.response;
      lastGeneratedPuzzle = { response, variant: request.variant };
      setGenerationProgress(request.max_attempts, request.max_attempts, true);
      setStatus("generate-status", response.warning ?? `Generated ${response.difficulty} puzzle`);
      renderBoard("generate-board", response.puzzle_line, response.region_line);
      renderBoard("generate-solution-board", response.solution_line, response.region_line);
      setOutput("generate-output", response);
    } else if (message.type === "error") {
      lastGeneratedPuzzle = undefined;
      setGenerationProgress(0, request.max_attempts, true);
      setStatus("generate-status", message.error);
      setOutput("generate-output", message.error);
    }
  });

  generationWorker.postMessage({ type: "generate", request });
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
  resetSolverProgress(lastGeneratedPuzzle.response.puzzle_line ?? "", lastGeneratedPuzzle.response.region_line);
  setStatus("solve-status", "Loaded generated puzzle");
  renderBoard("solve-board", solverBaselineLine, solverRegionLine);
  renderBoard("solve-solution-board", solverBaselineLine, solverRegionLine);
  setOutput("solve-output", "Loaded generated puzzle into solver input.");
});

byId("solve-clear-solution-button").addEventListener("click", () => {
  if (solverBaselineLine === undefined) {
    resetSolverProgress(byId("solve-puzzle").value, asOptionalText(byId("solve-region").value));
  } else {
    solverProgressLine = solverBaselineLine;
  }
  renderBoard("solve-solution-board", solverBaselineLine, solverRegionLine);
  setStatus("solve-status", "Solution cleared");
});

byId("solve-reset-button").addEventListener("click", () => {
  byId("solve-variant").value = "standard";
  byId("solve-format").value = "auto";
  byId("solve-puzzle").value = "";
  byId("solve-region").value = "";
  resetSolverProgress(undefined, undefined);
  setStatus("solve-status", "");
  renderBoard("solve-board");
  renderBoard("solve-solution-board");
  setOutput("solve-output", "");
});

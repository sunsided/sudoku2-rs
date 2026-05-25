import init, { generate_puzzle_with_callback } from "../pkg/sudoku2.js";

let ready;

async function ensureReady() {
  if (ready === undefined) {
    ready = init();
  }
  await ready;
}

self.addEventListener("message", async (event) => {
  if (event.data?.type !== "generate") {
    return;
  }

  try {
    await ensureReady();
    const response = generate_puzzle_with_callback(event.data.request, (progress) => {
      self.postMessage({ type: "progress", progress });
    });
    self.postMessage({ type: "complete", response });
  } catch (error) {
    self.postMessage({ type: "error", error: String(error) });
  }
});

/**
 * Client-side helper for the /api/hf proxy.
 *
 * Usage:
 *   import { hfInfer } from "@/lib/hf";
 *   const result = await hfInfer("bigcode/starcoder2-7b", sourceCode);
 */

const DEFAULT_MODEL = "bigcode/starcoder2-7b";

/**
 * Send `inputs` to a HuggingFace text-generation model via the server proxy.
 * Returns the raw response object from HF.
 *
 * @param {string} inputs   - The prompt string.
 * @param {object} [opts]
 * @param {string} [opts.model]       - HF model slug (default: starcoder2-7b).
 * @param {object} [opts.parameters]  - HF inference parameters.
 */
export async function hfInfer(inputs, { model = DEFAULT_MODEL, parameters } = {}) {
  const res = await fetch("/api/hf", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ model, inputs, parameters }),
  });

  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error || `HF proxy returned ${res.status}`);
  }

  return res.json();
}

/**
 * /api/hf — server-side proxy for the HuggingFace Inference API.
 *
 * The browser never sees HF_TOKEN. All requests from the wt DSL compiler
 * in the sandbox go through this route.
 *
 * Request body (POST):
 *   { model: string, inputs: string, parameters?: object }
 *
 * The response is the raw JSON from HF, forwarded as-is.
 */
export default async function handler(req, res) {
  if (req.method !== "POST") {
    return res.status(405).json({ error: "Method not allowed" });
  }

  const token =
    process.env.HF_TOKEN || process.env.HUGGINGFACE_HUB_TOKEN;

  if (!token) {
    return res.status(503).json({
      error:
        "HF_TOKEN is not configured. Add it in Vercel → Project Settings → Environment Variables.",
    });
  }

  const { model, inputs, parameters } = req.body;

  if (!model || !inputs) {
    return res.status(400).json({ error: "model and inputs are required" });
  }

  const hfUrl = `https://api-inference.huggingface.co/models/${model}`;

  try {
    const hfRes = await fetch(hfUrl, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ inputs, parameters }),
    });

    const contentType = hfRes.headers.get("content-type") || "";
    const body = contentType.includes("application/json")
      ? await hfRes.json()
      : await hfRes.text();

    return res.status(hfRes.status).json(
      typeof body === "string" ? { raw: body } : body
    );
  } catch (err) {
    return res.status(502).json({ error: `HF API unreachable: ${err.message}` });
  }
}

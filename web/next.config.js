/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,

  // HF_TOKEN and HUGGINGFACE_HUB_TOKEN are server-side only (no NEXT_PUBLIC_ prefix).
  // They are never embedded in the client bundle.
  // Set them in Vercel → Project Settings → Environment Variables.
  //
  // The /api/hf route reads process.env.HF_TOKEN server-side and proxies
  // requests to the HuggingFace Inference API, so the token is never
  // exposed to the browser.
  env: {},
};

module.exports = nextConfig;

import Head from "next/head";
import dynamic from "next/dynamic";

// GLB viewer must be client-only — three.js has no SSR support.
const WindTunnelModel = dynamic(
  () => import("@/components/WindTunnelModel"),
  { ssr: false }
);

export default function Home() {
  return (
    <>
      <Head>
        <title>Wind Tunnel</title>
        <meta
          name="description"
          content="Global self-consistency analysis for software."
        />
      </Head>

      {/* Full-viewport black canvas — the model is the page. */}
      <div className="fixed inset-0 bg-black">
        <WindTunnelModel />
      </div>
    </>
  );
}

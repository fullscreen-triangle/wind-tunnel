import Head from "next/head";
import Link from "next/link";
import dynamic from "next/dynamic";
import { useRouter } from "next/router";

const WindTunnelModel = dynamic(
  () => import("@/components/WindTunnelModel"),
  { ssr: false }
);

export default function Home() {
  return (
    <>
      <Head>
        <title>Wind Tunnel</title>
        <meta name="description" content="Global self-consistency analysis for software." />
      </Head>

      {/* Full-viewport black canvas */}
      <div style={{ position: "fixed", inset: 0, background: "#000" }}>
        <WindTunnelModel />
      </div>

      {/* Navbar floats above canvas — scoped to landing page only */}
      <header style={{
        position: "fixed", top: 0, left: 0, right: 0, zIndex: 10,
        display: "flex", alignItems: "center", justifyContent: "space-between",
        padding: "24px 40px",
      }}>
        <span style={{ color: "#fff", fontSize: 12, fontFamily: "monospace", letterSpacing: 4, textTransform: "uppercase", opacity: 0.6 }}>
          Wind Tunnel
        </span>
        <Link href="/sandbox" style={{
          color: "#fff", fontSize: 12, fontFamily: "monospace",
          letterSpacing: 4, textTransform: "uppercase", opacity: 0.5,
          textDecoration: "none",
        }}
          onMouseEnter={e => e.currentTarget.style.opacity = 1}
          onMouseLeave={e => e.currentTarget.style.opacity = 0.5}
        >
          Sandbox
        </Link>
      </header>
    </>
  );
}

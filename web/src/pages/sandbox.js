import Head from "next/head";
import SandboxIDE from "@/components/SandboxIDE";

export default function Sandbox() {
  return (
    <>
      <Head>
        <title>Wind Tunnel — Sandbox</title>
      </Head>
      <SandboxIDE />
    </>
  );
}

import Footer from "@/components/Footer";
import Navbar from "@/components/Navbar";
import "@/styles/globals.css";
import { Montserrat } from "next/font/google";
import Head from "next/head";
import { useRouter } from "next/router";

const montserrat = Montserrat({ subsets: ["latin"], variable: "--font-mont" });

export default function App({ Component, pageProps }) {
  const router = useRouter();
  const isHome = router.pathname === "/";

  return (
    <>
      <Head>
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <link rel="icon" href="/favicon.ico" />
      </Head>
      {/*
        On the landing page the canvas is fixed to the viewport, so the main
        wrapper must be transparent and have no min-height that would push
        a scrollbar into existence.
      */}
      <main
        className={`${montserrat.variable} font-mont ${
          isHome ? "bg-transparent" : "bg-light dark:bg-dark w-full min-h-screen"
        }`}
      >
        <Navbar />
        <Component {...pageProps} />
        <Footer />
      </main>
    </>
  );
}

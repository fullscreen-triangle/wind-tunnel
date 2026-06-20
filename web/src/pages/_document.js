import { Html, Head, Main, NextScript } from "next/document";

export default function Document() {
  return (
    // `dark` class is hardcoded — no runtime toggle, no flash.
    <Html lang="en" className="dark">
      <Head />
      <body>
        <Main />
        <NextScript />
      </body>
    </Html>
  );
}

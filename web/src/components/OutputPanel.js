import { useEffect, useRef, useState } from "react";

const TABS = ["Terminal", "Problems", "Output"];

const KIND_STYLE = {
  info:    { color: "#9cdcfe" },
  success: { color: "#4ec9b0" },
  warn:    { color: "#ce9178" },
  error:   { color: "#f48771" },
  metric:  { color: "#d4d4d4" },
};

const KIND_PREFIX = {
  info:    "ℹ ",
  success: "✔ ",
  warn:    "⚠ ",
  error:   "✖ ",
  metric:  "  ",
};

export default function OutputPanel({ lines, running }) {
  const [tab, setTab] = useState("Terminal");
  const bottomRef = useRef(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [lines]);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", background: "#1e1e1e" }}>

      {/* Panel tab bar */}
      <div style={{ display: "flex", alignItems: "center", height: 30, background: "#252526", borderBottom: "1px solid #111", flexShrink: 0 }}>
        {TABS.map((t) => (
          <button key={t} onClick={() => setTab(t)} style={{
            background: "transparent", border: "none",
            borderBottom: t === tab ? "1px solid #0e639c" : "1px solid transparent",
            color: t === tab ? "#fff" : "#858585",
            padding: "0 14px", height: "100%", fontSize: 12,
            cursor: "pointer", letterSpacing: 0.5,
          }}>
            {t}
          </button>
        ))}
        {running && (
          <span style={{ marginLeft: "auto", marginRight: 12, fontSize: 11, color: "#4ec9b0", animation: "pulse 1s infinite" }}>
            ● running
          </span>
        )}
      </div>

      {/* Content */}
      <div style={{ flex: 1, overflow: "auto", padding: "8px 12px", fontSize: 12, lineHeight: "20px" }}>
        {tab === "Terminal" && (
          <>
            {lines.length === 0 && (
              <span style={{ color: "#555" }}>Press ▶ Run to execute the script.</span>
            )}
            {lines.map((line, i) => (
              <div key={i} style={{ ...(KIND_STYLE[line.kind] || {}), whiteSpace: "pre" }}>
                {KIND_PREFIX[line.kind] || ""}{line.text}
              </div>
            ))}
            <div ref={bottomRef} />
          </>
        )}
        {tab === "Problems" && (
          <span style={{ color: "#555" }}>No problems detected.</span>
        )}
        {tab === "Output" && (
          <span style={{ color: "#555" }}>Run the script to see structured output.</span>
        )}
      </div>

      <style>{`@keyframes pulse { 0%,100%{opacity:1} 50%{opacity:0.4} }`}</style>
    </div>
  );
}

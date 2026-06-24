import { useRef, useCallback } from "react";

// Token patterns for the Wind Tunnel DSL.
const KEYWORDS  = /\b(scope|analyse|assert|report|repo|include|exclude|language|static|dynamic|purpose|cycles|through|max_depth|ablate|regime|no|none|format|json|in)\b/g;
const STRINGS   = /("(?:[^"\\]|\\.)*")/g;
const OPERATORS = /\b(Coherent|Phase-locked|Turbulent|Hierarchical|Aperture)\b/g;
const COMMENTS  = /(#.*$)/gm;
const NUMBERS   = /\b(\d+(?:\.\d+)?)\b/g;

function highlight(code) {
  // Escape HTML first, then apply colour spans.
  const escaped = code
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");

  return escaped
    .replace(COMMENTS,  '<span style="color:#6a9955">$1</span>')
    .replace(STRINGS,   '<span style="color:#ce9178">$1</span>')
    .replace(KEYWORDS,  '<span style="color:#569cd6">$&</span>')
    .replace(OPERATORS, '<span style="color:#4ec9b0">$&</span>')
    .replace(NUMBERS,   '<span style="color:#b5cea8">$&</span>');
}

export default function Editor({ value, onChange }) {
  const textareaRef = useRef(null);
  const highlightRef = useRef(null);

  // Keep the highlight layer scrolled in sync with the textarea.
  const syncScroll = useCallback(() => {
    if (highlightRef.current && textareaRef.current) {
      highlightRef.current.scrollTop  = textareaRef.current.scrollTop;
      highlightRef.current.scrollLeft = textareaRef.current.scrollLeft;
    }
  }, []);

  const handleTab = useCallback((e) => {
    if (e.key !== "Tab") return;
    e.preventDefault();
    const el    = e.target;
    const start = el.selectionStart;
    const end   = el.selectionEnd;
    const next  = value.substring(0, start) + "    " + value.substring(end);
    onChange(next);
    // Restore cursor after React re-render.
    requestAnimationFrame(() => {
      el.selectionStart = el.selectionEnd = start + 4;
    });
  }, [value, onChange]);

  const lines = value.split("\n").length;

  return (
    <div style={{ position: "relative", width: "100%", height: "100%", display: "flex", overflow: "hidden", background: "#1e1e1e" }}>

      {/* Line numbers */}
      <div style={{
        width: 44, flexShrink: 0, paddingTop: 14, paddingBottom: 14,
        textAlign: "right", paddingRight: 10, color: "#858585",
        fontSize: 13, lineHeight: "20px", userSelect: "none",
        background: "#1e1e1e", overflowY: "hidden",
      }}>
        {Array.from({ length: lines }, (_, i) => (
          <div key={i}>{i + 1}</div>
        ))}
      </div>

      {/* Highlight layer (behind) */}
      <pre
        ref={highlightRef}
        aria-hidden
        style={{
          position: "absolute", left: 44, top: 0, right: 0, bottom: 0,
          margin: 0, padding: "14px 14px 14px 10px",
          fontSize: 13, lineHeight: "20px", fontFamily: "inherit",
          color: "#d4d4d4", whiteSpace: "pre", overflowX: "auto",
          overflowY: "auto", pointerEvents: "none", tabSize: 4,
        }}
        dangerouslySetInnerHTML={{ __html: highlight(value) + "\n" }}
      />

      {/* Editable textarea (on top, transparent text so highlight shows) */}
      <textarea
        ref={textareaRef}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onScroll={syncScroll}
        onKeyDown={handleTab}
        spellCheck={false}
        style={{
          position: "absolute", left: 44, top: 0, right: 0, bottom: 0,
          margin: 0, padding: "14px 14px 14px 10px",
          fontSize: 13, lineHeight: "20px", fontFamily: "inherit",
          background: "transparent", color: "transparent",
          caretColor: "#aeafad", border: "none", outline: "none",
          resize: "none", whiteSpace: "pre", overflowX: "auto",
          overflowY: "auto", tabSize: 4,
        }}
      />
    </div>
  );
}

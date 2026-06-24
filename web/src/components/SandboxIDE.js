import { useState, useRef, useCallback, useEffect } from "react";

// ── DSL syntax highlighting ────────────────────────────────────────────────

function highlight(code) {
  const esc = code
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");

  return esc
    // comments
    .replace(/(#[^\n]*)/g, '<span style="color:#6a9955">$1</span>')
    // strings
    .replace(/("(?:[^"\\]|\\.)*")/g, '<span style="color:#ce9178">$1</span>')
    // block keywords
    .replace(/\b(scope|analyse|analyze|assert|report)\b/g, '<span style="color:#c586c0">$&</span>')
    // value keywords
    .replace(/\b(repo|include|exclude|language|static|dynamic|purpose|cycles|through|max_depth|ablate|format|json|no|none|in)\b/g, '<span style="color:#569cd6">$&</span>')
    // regime names
    .replace(/\b(Coherent|Phase-locked|Turbulent|Hierarchical|Aperture)\b/g, '<span style="color:#4ec9b0">$&</span>')
    // field names (word followed by colon at start of token)
    .replace(/\b(regime|r_est|holonomy_violations|cycle_graph|regime_map|contribution_scores)\b/g, '<span style="color:#9cdcfe">$&</span>')
    // numbers
    .replace(/\b(\d+(?:\.\d+)?)\b/g, '<span style="color:#b5cea8">$&</span>')
    // operators
    .replace(/(>=|<=|==|!=|>|<)/g, '<span style="color:#d4d4d4">$&</span>');
}

// ── Initial script ─────────────────────────────────────────────────────────

const INITIAL = `# Wind Tunnel DSL — directed analysis script

scope PaymentFlowAnalysis:
    repo "github.com/owner/repo"
    include "src/payments/**"
    include "src/ledger/**"
    exclude "**/__tests__/**"
    language typescript

analyse:
    static
    cycles through ["checkout", "ledger", "reconciler"] max_depth 5
    purpose ablate ["audit_log", "retry_middleware"]

assert:
    regime >= Coherent
    r_est >= 0.75
    no holonomy_violations
    purposeless none in ["checkout", "ledger"]

report:
    format json
    include regime_map
    include cycle_graph
    include contribution_scores
`;

// ── Colours ────────────────────────────────────────────────────────────────

const C = {
  bg:          "#1e1e1e",
  bgPanel:     "#252526",
  bgTab:       "#2d2d2d",
  bgTitlebar:  "#323233",
  bgActivity:  "#333",
  bgStatus:    "#007acc",
  border:      "#111",
  text:        "#ccc",
  textDim:     "#858585",
  textActive:  "#fff",
  accent:      "#0e639c",
  accentTeal:  "#4ec9b0",
  activeFile:  "#094771",
};

const MONO = "'Cascadia Code', 'Fira Code', 'Consolas', monospace";

// ── Editor component ───────────────────────────────────────────────────────

function Editor({ value, onChange }) {
  const taRef   = useRef(null);
  const preRef  = useRef(null);
  const lineRef = useRef(null);

  // Sync scroll across all three layers.
  const syncScroll = () => {
    const ta = taRef.current;
    if (!ta) return;
    if (preRef.current)  { preRef.current.scrollTop  = ta.scrollTop;  preRef.current.scrollLeft  = ta.scrollLeft; }
    if (lineRef.current) { lineRef.current.scrollTop = ta.scrollTop; }
  };

  const onKeyDown = (e) => {
    if (e.key === "Tab") {
      e.preventDefault();
      const ta    = e.target;
      const start = ta.selectionStart;
      const end   = ta.selectionEnd;
      const next  = value.slice(0, start) + "    " + value.slice(end);
      onChange(next);
      requestAnimationFrame(() => { ta.selectionStart = ta.selectionEnd = start + 4; });
    }
  };

  const lineCount = value.split("\n").length;

  const shared = {
    fontFamily: MONO,
    fontSize: 13,
    lineHeight: "20px",
    tabSize: 4,
    whiteSpace: "pre",
    padding: "12px 16px",
    margin: 0,
    overflowX: "auto",
    overflowY: "auto",
    boxSizing: "border-box",
  };

  return (
    <div style={{ display: "flex", width: "100%", height: "100%", overflow: "hidden", background: C.bg }}>

      {/* Line numbers — scroll locked to textarea */}
      <div ref={lineRef} style={{
        width: 48, flexShrink: 0, overflowY: "hidden",
        paddingTop: 12, paddingBottom: 12,
        textAlign: "right", paddingRight: 12,
        color: C.textDim, fontFamily: MONO, fontSize: 13, lineHeight: "20px",
        userSelect: "none", background: C.bg,
      }}>
        {Array.from({ length: lineCount }, (_, i) => (
          <div key={i + 1}>{i + 1}</div>
        ))}
      </div>

      {/* Editor stack: highlight pre behind, textarea on top */}
      <div style={{ position: "relative", flex: 1, overflow: "hidden" }}>

        {/* Highlight layer */}
        <pre
          ref={preRef}
          aria-hidden="true"
          style={{
            ...shared,
            position: "absolute", inset: 0,
            color: "#d4d4d4",
            pointerEvents: "none",
            zIndex: 1,
            wordBreak: "keep-all",
          }}
          dangerouslySetInnerHTML={{ __html: highlight(value) + "\n" }}
        />

        {/* Editable layer — sits on top, text is transparent so highlight shows */}
        <textarea
          ref={taRef}
          value={value}
          onChange={e => onChange(e.target.value)}
          onScroll={syncScroll}
          onKeyDown={onKeyDown}
          spellCheck={false}
          autoComplete="off"
          autoCorrect="off"
          autoCapitalize="off"
          style={{
            ...shared,
            position: "absolute", inset: 0,
            background: "transparent",
            color: "transparent",
            caretColor: "#aeafad",
            border: "none",
            outline: "none",
            resize: "none",
            zIndex: 2,          // above the highlight pre
            width: "100%",
            height: "100%",
          }}
        />
      </div>
    </div>
  );
}

// ── Output panel ───────────────────────────────────────────────────────────

const KIND = {
  info:    { color: "#9cdcfe", pre: "ℹ  " },
  success: { color: "#4ec9b0", pre: "✔  " },
  warn:    { color: "#ce9178", pre: "⚠  " },
  error:   { color: "#f48771", pre: "✖  " },
  metric:  { color: "#d4d4d4", pre: "   " },
};

function OutputPanel({ lines, running }) {
  const [tab, setTab] = useState("Terminal");
  const endRef = useRef(null);
  useEffect(() => { endRef.current?.scrollIntoView({ behavior: "smooth" }); }, [lines]);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", background: C.bg }}>
      {/* Tab bar */}
      <div style={{ display: "flex", alignItems: "center", height: 30, background: C.bgPanel, borderBottom: `1px solid ${C.border}`, flexShrink: 0 }}>
        {["Terminal", "Problems", "Output"].map(t => (
          <button key={t} onClick={() => setTab(t)} style={{
            background: "transparent", border: "none",
            borderBottom: t === tab ? `1px solid ${C.accent}` : "1px solid transparent",
            color: t === tab ? C.textActive : C.textDim,
            padding: "0 14px", height: "100%", fontSize: 12,
            cursor: "pointer", fontFamily: MONO,
          }}>
            {t}
          </button>
        ))}
        {running && (
          <span style={{ marginLeft: "auto", marginRight: 12, fontSize: 11, color: C.accentTeal }}>
            ● running
          </span>
        )}
      </div>

      {/* Body */}
      <div style={{ flex: 1, overflowY: "auto", padding: "8px 14px", fontFamily: MONO, fontSize: 12, lineHeight: "20px" }}>
        {tab === "Terminal" && (
          lines.length === 0
            ? <span style={{ color: C.textDim }}>Press ▶ Run to execute the script.</span>
            : lines.map((l, i) => (
                <div key={i} style={{ color: KIND[l.kind]?.color || C.text, whiteSpace: "pre" }}>
                  {KIND[l.kind]?.pre}{l.text}
                </div>
              ))
        )}
        {tab === "Problems" && <span style={{ color: C.textDim }}>No problems detected.</span>}
        {tab === "Output"   && <span style={{ color: C.textDim }}>Run the script to see structured output.</span>}
        <div ref={endRef} />
      </div>
    </div>
  );
}

// ── Sidebar ────────────────────────────────────────────────────────────────

const FILES = [
  { name: "analysis.wt",     active: true  },
  { name: "payment-flow.wt", active: false },
  { name: "auth-cycle.wt",   active: false },
];

function Sidebar() {
  const [open, setOpen] = useState(true);
  return (
    <div style={{ fontSize: 12, fontFamily: MONO, color: C.text, overflowY: "auto" }}>
      <div
        onClick={() => setOpen(o => !o)}
        style={{ padding: "6px 12px", color: C.textDim, letterSpacing: 1, textTransform: "uppercase", fontSize: 11, cursor: "pointer", userSelect: "none" }}
      >
        {open ? "▾" : "▸"} &nbsp;Scripts
      </div>
      {open && FILES.map(f => (
        <div key={f.name} style={{
          padding: "3px 12px 3px 24px",
          background: f.active ? C.activeFile : "transparent",
          color: f.active ? C.textActive : C.text,
          cursor: "pointer", display: "flex", alignItems: "center", gap: 6,
        }}>
          <span style={{ color: C.accentTeal, fontSize: 10 }}>⬡</span>
          {f.name}
        </div>
      ))}
    </div>
  );
}

// ── Main IDE ───────────────────────────────────────────────────────────────

export default function SandboxIDE() {
  const [script,       setScript]       = useState(INITIAL);
  const [output,       setOutput]       = useState([]);
  const [running,      setRunning]      = useState(false);
  const [sidebarW,     setSidebarW]     = useState(200);
  const [outputH,      setOutputH]      = useState(220);

  // ── Drag: sidebar width ──────────────────────────────────────────────────
  const startColDrag = (e) => {
    e.preventDefault();
    const x0 = e.clientX, w0 = sidebarW;
    const move = ev => setSidebarW(Math.max(120, Math.min(400, w0 + ev.clientX - x0)));
    const up   = ()  => { window.removeEventListener("mousemove", move); window.removeEventListener("mouseup", up); };
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", up);
  };

  // ── Drag: output height ──────────────────────────────────────────────────
  const startRowDrag = (e) => {
    e.preventDefault();
    const y0 = e.clientY, h0 = outputH;
    const move = ev => setOutputH(Math.max(60, Math.min(500, h0 - (ev.clientY - y0))));
    const up   = ()  => { window.removeEventListener("mousemove", move); window.removeEventListener("mouseup", up); };
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", up);
  };

  // ── Run ──────────────────────────────────────────────────────────────────
  const run = async () => {
    setRunning(true);
    setOutput([{ kind: "info", text: "Compiling script…" }]);
    await delay(500);
    setOutput(o => [...o, { kind: "info", text: "Fetching repository file tree…" }]);
    await delay(700);
    setOutput(o => [...o, { kind: "info", text: "Extracting dependency graph (tree-sitter)…" }]);
    await delay(900);
    setOutput(o => [...o,
      { kind: "success", text: "Graph built: 47 units, 83 edges." },
      { kind: "info",    text: "Running static analysis…" },
    ]);
    await delay(600);
    setOutput(o => [...o,
      { kind: "metric",  text: "Regime        : Hierarchical cascade" },
      { kind: "metric",  text: "R_est         : 0.6143" },
      { kind: "metric",  text: "K_c           : 0.8821" },
      { kind: "metric",  text: "Decoherence zones : 2" },
      { kind: "metric",  text: "Cycle candidates  : 1" },
      { kind: "metric",  text: '  ["checkout","ledger","reconciler"]  residual=2.34' },
      { kind: "warn",    text: "Assert failed: regime >= Coherent  (0.6143 < 0.80)" },
      { kind: "warn",    text: "Assert failed: r_est >= 0.75  (0.6143 < 0.75)" },
    ]);
    setRunning(false);
  };

  return (
    <div style={{
      display: "flex", flexDirection: "column",
      width: "100vw", height: "100vh",
      background: C.bg, color: C.text,
      overflow: "hidden", fontFamily: MONO,
    }}>

      {/* ── Title bar ─────────────────────────────────────────────────────── */}
      <div style={{
        height: 30, flexShrink: 0,
        background: C.bgTitlebar,
        borderBottom: `1px solid ${C.border}`,
        display: "flex", alignItems: "center",
        justifyContent: "space-between",
        padding: "0 14px",
      }}>
        <span style={{ fontSize: 11, color: C.textDim, letterSpacing: 3, textTransform: "uppercase" }}>
          Wind Tunnel
        </span>
        <button
          onClick={run}
          disabled={running}
          style={{
            background: "transparent",
            border: `1px solid ${C.accentTeal}`,
            color: C.accentTeal,
            padding: "2px 14px", fontSize: 11,
            borderRadius: 2, cursor: running ? "not-allowed" : "pointer",
            opacity: running ? 0.5 : 1, letterSpacing: 1,
            fontFamily: MONO,
          }}
        >
          {running ? "running…" : "▶  Run"}
        </button>
      </div>

      {/* ── Main row ──────────────────────────────────────────────────────── */}
      <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>

        {/* Activity bar */}
        <div style={{ width: 44, flexShrink: 0, background: C.bgActivity, display: "flex", flexDirection: "column", alignItems: "center", paddingTop: 8, gap: 4 }}>
          {[
            { label: "Explorer", icon: "☰" },
            { label: "Search",   icon: "⌕" },
          ].map(({ label, icon }) => (
            <button key={label} title={label} style={{
              width: 38, height: 38, background: "transparent", border: "none",
              color: label === "Explorer" ? C.textActive : C.textDim,
              fontSize: 18, cursor: "pointer",
              borderLeft: label === "Explorer" ? `2px solid ${C.textActive}` : "2px solid transparent",
            }}>
              {icon}
            </button>
          ))}
        </div>

        {/* Sidebar */}
        <div style={{ width: sidebarW, flexShrink: 0, background: C.bgPanel, borderRight: `1px solid ${C.border}`, overflowY: "auto" }}>
          <Sidebar />
        </div>

        {/* Drag handle — col */}
        <div
          onMouseDown={startColDrag}
          style={{ width: 4, flexShrink: 0, cursor: "col-resize", background: "transparent" }}
          onMouseEnter={e => e.currentTarget.style.background = C.accent}
          onMouseLeave={e => e.currentTarget.style.background = "transparent"}
        />

        {/* Editor + output column */}
        <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>

          {/* Tab bar */}
          <div style={{ height: 35, flexShrink: 0, background: C.bgTab, borderBottom: `1px solid ${C.border}`, display: "flex", alignItems: "center" }}>
            <div style={{
              height: "100%", padding: "0 16px",
              display: "flex", alignItems: "center", gap: 8,
              fontSize: 12, color: C.textActive,
              borderBottom: `1px solid ${C.accent}`,
            }}>
              <span style={{ color: C.accentTeal }}>⬡</span>
              analysis.wt
              <span style={{ color: C.textDim, fontSize: 10 }}>●</span>
            </div>
          </div>

          {/* Editor */}
          <div style={{ flex: 1, overflow: "hidden" }}>
            <Editor value={script} onChange={setScript} />
          </div>

          {/* Drag handle — row */}
          <div
            onMouseDown={startRowDrag}
            style={{ height: 4, flexShrink: 0, cursor: "row-resize", background: "transparent" }}
            onMouseEnter={e => e.currentTarget.style.background = C.accent}
            onMouseLeave={e => e.currentTarget.style.background = "transparent"}
          />

          {/* Output */}
          <div style={{ height: outputH, flexShrink: 0, borderTop: `1px solid ${C.border}` }}>
            <OutputPanel lines={output} running={running} />
          </div>
        </div>
      </div>

      {/* ── Status bar ────────────────────────────────────────────────────── */}
      <div style={{
        height: 22, flexShrink: 0,
        background: C.bgStatus,
        display: "flex", alignItems: "center",
        padding: "0 12px", gap: 16,
        fontSize: 11, color: "#fff",
      }}>
        <span>Wind Tunnel DSL</span>
        <span style={{ opacity: 0.7 }}>analysis.wt</span>
        <span style={{ marginLeft: "auto", opacity: 0.7 }}>UTF-8</span>
      </div>
    </div>
  );
}

function delay(ms) { return new Promise(r => setTimeout(r, ms)); }

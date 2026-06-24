import { useState } from "react";

const TREE = [
  {
    name: "scripts",
    type: "dir",
    open: true,
    children: [
      { name: "analysis.wt",      type: "file", active: true },
      { name: "payment-flow.wt",  type: "file" },
      { name: "auth-cycle.wt",    type: "file" },
    ],
  },
  {
    name: "results",
    type: "dir",
    open: false,
    children: [
      { name: "output.json", type: "file" },
    ],
  },
];

function FileIcon({ type, isOpen }) {
  if (type === "dir") {
    return <span style={{ fontSize: 10, marginRight: 4, opacity: 0.7 }}>{isOpen ? "▾" : "▸"}</span>;
  }
  return <span style={{ fontSize: 10, marginRight: 4, color: "#4ec9b0" }}>⬡</span>;
}

export default function SidebarExplorer() {
  const [tree, setTree] = useState(TREE);

  const toggleDir = (name) => {
    setTree((t) => t.map((n) => n.name === name ? { ...n, open: !n.open } : n));
  };

  return (
    <div style={{ fontSize: 13, overflow: "auto", flex: 1, paddingBottom: 8 }}>
      {tree.map((node) => (
        <div key={node.name}>
          <div
            onClick={() => node.type === "dir" && toggleDir(node.name)}
            style={{
              display: "flex", alignItems: "center", padding: "2px 12px",
              cursor: "pointer", color: "#ccc",
              userSelect: "none",
            }}
            onMouseEnter={(e) => e.currentTarget.style.background = "#2a2d2e"}
            onMouseLeave={(e) => e.currentTarget.style.background = "transparent"}
          >
            <FileIcon type={node.type} isOpen={node.open} />
            <span>{node.name}</span>
          </div>
          {node.type === "dir" && node.open && node.children.map((child) => (
            <div
              key={child.name}
              style={{
                display: "flex", alignItems: "center",
                padding: "2px 12px 2px 28px",
                cursor: "pointer",
                color: child.active ? "#fff" : "#ccc",
                background: child.active ? "#094771" : "transparent",
              }}
              onMouseEnter={(e) => { if (!child.active) e.currentTarget.style.background = "#2a2d2e"; }}
              onMouseLeave={(e) => { if (!child.active) e.currentTarget.style.background = "transparent"; }}
            >
              <FileIcon type={child.type} />
              <span>{child.name}</span>
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}

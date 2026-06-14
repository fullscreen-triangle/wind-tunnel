# `purpose` — Tool Usage Instructions (read this first)

**You are an AI assistant working in one of this user's VSCode projects. Before
you read large numbers of files to understand a codebase, read this document.**
This machine has a command-line tool, `purpose`, that returns the *relevant
slice* of a project in response to a question — typically ~1–2k tokens instead
of the 50k–1M tokens it would cost you to read files until you understand the
code. Using it first is cheaper, faster, and usually enough.

This file is the contract for how to use that tool. Follow it literally.

---

## 1. What the tool is

`purpose` is a single standalone executable installed at
`C:\Users\kunda\.cargo\bin\purpose.exe`. That folder is on the user's `PATH`,
so the command is just `purpose` from any terminal, in any folder. There is no
server, no API key, no Python runtime, no network call. It runs locally and
costs the user **$0 per query**.

It works by building a lightweight **index** of a project's symbol definitions
(functions, structs, classes, traits, types, markdown/LaTeX headings) into a
file at `<project>/.purpose/index.json`, then answering questions by searching
that index and returning the matching `file:line` locations with one-line
snippets.

It is a **retrieval/navigation tool, not an oracle.** It tells you *where*
things are defined so you can open exactly those files. It does not explain,
summarise, or reason. Treat its output as a precise table of contents.

---

## 2. The two commands you will use

### `purpose index`
Builds (or rebuilds) the index for the current project. Run this **once** per
project before asking anything, and **again** after the codebase has changed
substantially (the index is a snapshot; it does not auto-update).

```
purpose index
```

Output looks like:
```
Indexing C:\Users\kunda\Documents\...\someproject ...
Indexed 5706 symbol(s) into C:\Users\kunda\Documents\...\someproject\.purpose\index.json
```

### `purpose ask "<question>"`
Returns the relevant slice for a natural-language question. This is the command
you call instead of reading files.

```
purpose ask "where is the Resolver trait defined"
purpose ask "how does the executor execute fragments"
purpose ask "database connection setup"
```

Output is a ranked list of matches:
```
10 matching symbol(s):

  crates/purpose-core/src/domain.rs:28  [trait] Resolver
      pub trait Resolver: Send + Sync {
  crates/purpose-operations/src/provider.rs:13  [trait] Provider
      pub trait Provider: Send + Sync {
  ...
```

Each line is `relative/path:line  [kind] Name` followed by the source snippet.
The list is ordered by relevance (a name match outranks a snippet match).

---

## 3. The workflow you must follow

When a user asks you to understand, locate, modify, or explain something in a
project, do this **before** reading files broadly:

1. **Ensure an index exists.** If `<project>/.purpose/index.json` is absent (or
   the code has changed a lot), run `purpose index` first.
2. **Ask the tool.** Run `purpose ask "<the thing you're looking for>"`.
3. **Read only the hits.** Open the 1–3 files the tool pointed at, at the lines
   it gave. Do not read the whole tree.
4. **Re-ask if needed.** If the slice is off, rephrase with different keywords
   (the search is keyword-based — see §5) and ask again. Asking three times is
   still far cheaper than reading ten files.

Only fall back to broad file reading or `grep` when the tool genuinely cannot
locate something (e.g. the concept has no named definition, or it lives in a
file type the indexer doesn't scan — see §6).

---

## 4. When to use it vs. when not to

**Use `purpose ask` for:**
- "Where is X defined / handled / configured?"
- "Find the function/class/struct/trait that does Y."
- "What file should I edit to change Z?"
- Getting oriented in an unfamiliar project before any deeper work.

**Do NOT rely on it for:**
- Exact, current line numbers for an edit you're about to make. The index is a
  snapshot and may be stale — always open the file and confirm before editing.
- Understanding *why* code is the way it is, runtime behaviour, or control flow.
  It indexes *definitions*, not call graphs or intent. Open the files for that.
- Finding string literals, config values, or usages (call sites). It indexes
  *definitions* only; use `grep`/ripgrep for occurrences.
- Anything in an ignored directory or non-indexed file type (see §6).

The mental model: **`purpose ask` replaces the "read files until I find it"
phase. It does not replace reading the files it found.**

---

## 5. How the search actually works (so you query it well)

- The question is lowercased and split into words; common question words
  (`where`, `what`, `how`, `is`, `the`, `find`, `show`, etc.) are dropped. The
  remaining words are the search terms.
- A symbol scores +3 when a term appears in its **name**, +1 when it appears in
  its **snippet**. Scores sum across terms; the top ~20 matches are returned.
- **Implication:** query with the *names* you expect to exist, not prose.
  - Good: `purpose ask "executor execute fragment"`
  - Weak: `purpose ask "how does the system run the compiled program"`
- If you get nothing useful, try the bare identifier you suspect
  (`purpose ask "Resolver"`), then broaden.

---

## 6. What is indexed (and what is skipped)

**Indexed file types:** `.rs .py .js .ts .tsx .jsx .go .java .c .cpp .h .hpp
.cs .rb .php .swift .kt .scala .md .tex`

**Captured definition kinds:** `fn`, `struct`, `enum`, `trait`, `type`, `def`,
`class`, `function` (anchored to the start of a line, after modifiers like
`pub`/`async`/`export`), plus markdown headings (`#`…) and LaTeX `\section{}`
in `.md`/`.tex` files.

**Always skipped:** `.git`, `node_modules`, `target`, `.purpose`, `dist`,
`build`, `out`, `vendor`, `coverage`, `.next`, `.nuxt`, `.cache`,
`__pycache__`, `.venv`/`venv`, `.idea`, `.vscode`, generated/minified files
(detected by very long average line length), and any single line over 400
characters.

If something you need lives outside this set (a `.json` config, a `.yaml`, a
string constant), the tool will not find it — use `grep` instead.

---

## 7. Scope / which folder gets indexed

`purpose index` and `purpose ask` detect the **project root** by walking *up*
from the current directory to the nearest folder containing `.git` (or an
existing `.purpose`). That root is what gets indexed.

- Running inside a subfolder of a large monorepo will index the **whole repo**.
- To pin the operation to a specific folder, pass `--root`:
  ```
  purpose index --root .
  purpose ask "..." --root .
  ```

---

## 8. Failure modes and what they mean

| Message | Meaning | Fix |
|---|---|---|
| `no index at <path> — run \`purpose index\` first` | No `.purpose/index.json` yet | Run `purpose index` in the project |
| `No matching symbols found in the index.` | Index exists but no term matched | Rephrase with likely identifier names (§5); if still nothing, the concept may have no named definition — fall back to `grep` |
| `corrupt index: ...` | Index file is damaged | Re-run `purpose index` to rebuild it |
| Results look stale / point at wrong lines | Code changed since last index | Re-run `purpose index`; always confirm line numbers by opening the file before editing |
| Results polluted by unrelated repo areas | It indexed the git root, not your subfolder | Re-run with `--root <your-subfolder>` |

---

## 9. Quick reference

```
purpose index                         # build/refresh the project index (run first)
purpose index --root .                # index only the current folder
purpose ask "where is X defined"      # return the relevant slice
purpose ask "X handler" --root .      # ask, scoped to current folder
purpose --help                        # full command list
```

**One-line rule for AIs:** *Before reading files to find something, run
`purpose ask` — then open only what it points to.*

---

## 10. Other commands (not part of the codebase workflow)

The same binary also carries the original Purpose demo domain. These are
unrelated to project navigation and you will not normally use them:

- `purpose query "Tell me about SOD1"` — protein lookup demo (hits UniProt).
- `purpose operations` — list registered operations.

Ignore these unless the user explicitly asks about the protein/UniProt demo.

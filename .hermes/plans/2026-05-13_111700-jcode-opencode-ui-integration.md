# jcode-plus: jcode backend + opencode UI — Integration Plan

Status: Planning
Created: 2026-05-13 11:17

## Goal

Combine **jcode's Rust backend** (agent runtime, providers, tools, MCP, sessions, protocol) with **opencode's terminal TUI** (command palette, session management, agent management, config UI, polished UX) into a single product — jcode-plus — that delivers the performance of jcode with the UX quality of opencode.

## Current State

### jcode (us — this repo)
- **Language**: Rust (edition 2024)
- **Backend**: Agent runtime, providers (OpenAI, Gemini, OpenRouter, Bedrock), MCP, tools, embedding, compaction, swarm, ambient tasks
- **Protocol**: `src/protocol.rs` — newline-delimited JSON over Unix socket (local daemon/server model)
- **Frontend (current)**: Ratatui TUI (~144 files, ~115K lines in `src/tui/`), WS server (`jcode-web-server`), mobile SwiftUI app
- **Desktop plan**: Custom wgpu-rendered Niri-like workspace superapp (docs exist, not built)
- **Strengths**: Insane speed (Rust, jemalloc, release-lto), rich tool ecosystem, strong protocol, server/session model
- **Weakness**: TUI UX less polished than opencode, complex codebase, UI is ratatui-dependent

### opencode (external)
- **Language**: TypeScript/Node.js (distributed as bundled JS)
- **Frontend**: Terminal TUI with rich command palette, session picker, agent management, provider management, model switching, web interface (`opencode web`)
- **Backend**: Node.js runtime, HTTP + ACP protocol, session management
- **Strengths**: Polished TUI UX, interactive command palette, great onboarding, web interface
- **Weakness**: Node.js runtime overhead, slower tool execution, JS-based agent loop

## Core Architecture Decision

**Two viable approaches:**

### Option A: jcode server + opencode client (Full Merge)
- jcode runs as a background daemon/server (already has server architecture)
- opencode's TUI connects to jcode via protocol (Unix socket / HTTP / WS)
- opencode becomes a thin UI shell sending commands to jcode backend
- **Pro**: Full jcode speed, full opencode UI
- **Con**: Requires adapting opencode's JS code to talk to jcode protocol, two codebases to maintain

### Option B: jcode TUI rewrite with opencode UX patterns (Pure Rust)
- Rewrite jcode's TUI from scratch alongside existing ratatui TUI
- Implement opencode-like UX patterns natively in Rust (command palette, session management, etc.)
- Leverage jcode's existing server protocol and all backend crates
- **Pro**: Single codebase, pure Rust, full control, no JS dependency
- **Con**: More dev effort, but jcode already has all the hard parts built

### Recommendation: Option B
Given jcode already has:
- A working server protocol
- A working web server (axum + WS)
- All the UX primitives distributed across 20+ TUI crates that can be reused
- Existing plans for TUI enhancement (command palette, session picker)

The most maintainable path is building the opencode UX inside jcode as a new TUI client that shares the backend protocol.

## Phased Plan

### Phase 1: Protocol Stabilization & Client-Core Split (1-2 weeks)

**Goal**: Make the server protocol robust enough to support a new TUI client

**From existing plan**: `docs/CLIENT_CORE_PRESENTATION_SPLIT_PLAN.md`

1. Complete the client-core vs presentation split already documented
2. Extract a `jcode-client-core` crate with clean state model, reducers, and protocol types
3. Make the existing ratatui TUI a consumer of `jcode-client-core` (verify no regression)
4. Strengthen the Unix socket protocol to support all opencode-like operations
5. Add missing protocol events: session list, session fork, provider list, model list/switch, agent management, config read/write

**Key files**:
- `crates/jcode-client-core/` (new crate)
- `src/protocol.rs` → expand event types
- `src/server/` → expand server capabilities
- `src/tui/app.rs` → refactor to use client-core

### Phase 2: New TUI Foundation (2-3 weeks)

**Goal**: Build opencode-quality TUI components in Rust with ratatui

**opencode UX features to implement**:

1. **Command Palette** (`/` prefix or `Cmd+K`)
   - Fuzzy search over all commands (sessions, agents, models, settings)
   - Keyboard-first navigation
   - Already partially exists: `crates/jcode-tui-command-palette/`

2. **Session Management**
   - Session list with previews (`opencode session`)
   - Session picker (`opencode attach`)
   - Continue/fork semantics
   - Already partially exists: `crates/jcode-tui-session-picker/`

3. **Agent & Model Management**
   - Model switching inline within session (`/model deepseek-v4-pro`)
   - Provider management (`/providers`)
   - Agent selection and configuration
   - Model list with pricing/token info

4. **Workspace / Multi-session**
   - Niri-like spatial navigation (scrolling columns)
   - Multiple concurrent sessions visible
   - Based on existing: `crates/jcode-tui-workspace/`

5. **Sidebar Navigation**
   - Session list drawer
   - Activity/task monitor
   - File/diff viewer
   - Based on existing: `crates/jcode-tui-sidebar/`

6. **Header / Status Bar**
   - Current model, session, context usage
   - Background task indicators
   - Token usage stats (`opencode stats`)

7. **Input Experience**
   - Rich multiline input with markdown preview
   - File/image drop/attach
   - `/slash` command autocomplete
   - Thinking mode indicator

**Key files** (new/expanded crates):
- `crates/jcode-tui-command-palette/` → enhance
- `crates/jcode-tui-session-picker/` → enhance
- `crates/jcode-tui-sidebar/` → enhance for navigation
- `crates/jcode-tui-workspace/` → spatial layout
- `crates/jcode-tui-messages/` → message rendering polish
- `crates/jcode-tui-style/` → opencode-like theme
- New: `crates/jcode-tui-input/` → rich input experience
- New: `crates/jcode-tui-header/` → status bar

### Phase 3: UI Polish & UX Parity (1-2 weeks)

**Goal**: Make the TUI feel like opencode

1. **Visual Theme**: Implement opencode-like color scheme, spacing, fonts
2. **Transitions**: Smooth view transitions (session picker ↔ chat, sidebar toggle)
3. **Keyboard Bindings**: Vim-style navigation (h/j/k/l), opencode shortcuts
4. **Inline Tool Display**: Show tool calls/results with collapsible sections
5. **Diff View**: Inline diff rendering for code changes
6. **Markdown Rendering**: Upgrade terminal markdown with syntax highlighting, diagrams (mermaid)
7. **Progress Indicators**: Spinners for agent thinking, tool execution
8. **Notifications**: Background task completion, errors, CI results

### Phase 4: Web Interface (1 week)

**Goal**: `jcode web` parity with `opencode web`

1. Enhance `jcode-web-server` with WS-based real-time updates
2. Build Svelte-based (or plain HTML/JS) web UI matching opencode's web interface
3. Session list, chat, settings, provider management
4. Mobile-first responsive design

### Phase 5: Performance Optimization (ongoing)

**Goal**: Never lose jcode's speed advantage

1. Virtualized rendering (only render visible messages)
2. Stream buffer optimization
3. Lazy markdown rendering
4. Frame budget monitoring (target: 60fps scroll, <16ms per frame)
5. Memory budget: <200MB idle, <500MB active session

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────┐
│                    jcode-plus CLI                        │
│  jcode [project]                jcode session list       │
│  jcode web                      jcode agent list         │
│  jcode run "message"            jcode providers          │
│  jcode attach <url>             jcode models             │
│  jcode --continue               jcode stats              │
│  jcode --model provider/model   jcode export/import      │
└─────────────────────────────────────────────────────────┘
                            │
                    ┌───────┴────────┐
                    │   CLI Args     │
                    │   (clap)       │
                    └───────┬────────┘
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│  TUI Client   │  │  Web Server   │  │  Headless     │
│  (ratatui)    │  │  (axum + WS)  │  │  (CLI mode)   │
│               │  │               │  │               │
│  Command      │  │  Web UI       │  │  Single       │
│  Palette      │  │  (HTML/JS)    │  │  message      │
│  Session Mgr  │  │               │  │  execution    │
│  Workspace    │  │               │  │               │
└───────┬───────┘  └───────┬───────┘  └───────┬───────┘
        │                  │                  │
        └──────────────────┼──────────────────┘
                           │
                    ┌──────┴───────┐
                    │  Protocol    │
                    │  (Unix sock) │
                    │  JSON-ND     │
                    └──────┬───────┘
                           │
                    ┌──────┴───────┐
                    │  Server /    │
                    │  Daemon      │
                    │              │
                    │  Sessions    │
                    │  Providers   │
                    │  Tools       │
                    │  MCP         │
                    │  Embedding   │
                    │  Memory      │
                    │  Compaction  │
                    │  Swarm       │
                    │  Background  │
                    └──────────────┘
```

## Component Ownership (Current vs Target)

| Component | Current | Target |
|-----------|---------|--------|
| Agent runtime | jcode (Rust) | jcode (unchanged) |
| Providers | jcode (Rust) | jcode (unchanged) |
| Tools | jcode (Rust) | jcode (unchanged) |
| MCP | jcode (Rust) | jcode (unchanged) |
| Protocol | jcode (Rust) | jcode (expand) |
| Session mgmt | jcode (Rust) | jcode (expand) |
| TUI rendering | jcode (ratatui) | jcode (ratatui, enhanced) |
| Command palette | jcode (partial) | jcode (full) |
| Session picker | jcode (partial) | jcode (full) |
| Web server | jcode (basic) | jcode (full) |
| Config UI | jcode (none) | jcode (new) |

## Key Risks

1. **ratatui limitations**: Terminal rendering may not reach opencode-quality animations/transitions. Mitigation: Accept ratatui's limits for now; desktop app with wgpu will come later per existing desktop plan.
2. **Codebase complexity**: jcode is already ~115K lines of TUI code. Adding opencode-like features must not create a maintenance monster. Mitigation: Complete the client-core split first.
3. **Two TUI modes**: Users may be confused by old TUI vs new TUI. Mitigation: Make new UX the default, old UX accessible via `--legacy` flag.
4. **Scope creep**: opencode has many features (ACP, MCP GUI, plugins, web). Mitigation: Phase delivery, core UX first, extras later.

## Success Metrics

- [ ] `jcode --continue` opens polished TUI with session picker
- [ ] `/model deepseek-v4-pro` switches model mid-session
- [ ] `jcode session list` shows all sessions with previews
- [ ] `jcode web` opens web interface with full session management
- [ ] Command palette (`Cmd+K`) fuzzy-searches all actions
- [ ] Session scrolling is 60fps (virtualized rendering)
- [ ] Memory usage < 200MB idle
- [ ] Startup time < 200ms cold, < 50ms warm

## Previous Art

- `docs/CLIENT_CORE_PRESENTATION_SPLIT_PLAN.md` — prerequisite refactor
- `docs/DESKTOP_APP_ARCHITECTURE.md` — long-term desktop direction (complementary, not competing)
- `docs/MULTI_SESSION_CLIENT_ARCHITECTURE.md` — multi-session support
- `docs/SERVER_ARCHITECTURE.md` — server/daemon model
- `docs/CRATE_OWNERSHIP_BOUNDARIES.md` — crate API boundaries

## Next Steps After Plan Approval

1. Start Phase 1: Complete client-core/presentation split
2. Implement protocol extensions for session/agent/model management
3. Begin Phase 2 TUI components (command palette first)

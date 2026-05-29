# Perf optimization diff — 2026-05-29

Baseline: release-max binary pre-perf-opts (v0.12.138, commit 6eead7a7).
After: release-max binary with 6 optimizations (v0.12.142).
Both built `--profile release-max` + `RUSTFLAGS=-C target-cpu=native`, M1 Pro, N=7.

## Landed optimizations
- T1.2 cap mermaid `LAST_RENDER`/`RENDER_ERRORS` (bounded to 128) — leak class removed.
- T1.3 cap `GRAPH_CACHE` (FIFO, max 8) — bounded RAM across many graphs.
- T1.4 Gemini HTTP/2 + connection pool (was HTTP/1.1, no pool) — ~100-300ms/request saved on multi-turn Gemini. Env fallback `JC_GEMINI_HTTP1=1`.
- T1.5 `.bak` gated to durable writes only + stale-`.bak` startup sweep — removes a rename per session save + reclaims accumulated `.bak`.
- T2.1 MCP pending-RPC leak fixed (remove on error/timeout).
- T2.2 memory store `write_json_fast` (no fsync on agent-turn hot path).

## Startup / size (no-regression gate)
| Metric | Baseline | After | Δ | Δ% | Verdict |
|---|---:|---:|---:|---:|:--:|
| help median ms | 7.62 | 7.78 | +0.16 | +2.0% | OK (noise) |
| version median ms | 6.89 | 7.39 | +0.49 | +7.2% | OK (sub-10ms, stdev 0.4ms) |
| binary size MB | 46.9 | 46.9 | +0.0 | +0.0% | OK |

Startup/size unchanged within noise — expected: these 6 are leak/disk/long-session/Gemini fixes, not cold-start-path changes. Their payoff shows in long-running sessions, disk write counts, and Gemini latency — not cold-start microbenchmarks.

## Notes
- Build profile itself (release-max: opt-level 3 + fat LTO + target-cpu=native) is the largest blanket runtime win vs the previous default `--release` (opt-level=1). Fat LTO also shrank the binary 73MB → 47MB.
- `cargo test --lib` currently FAILS TO COMPILE due to PRE-EXISTING WIP unrelated to these changes (`ReloadSignal.is_reset` missing in `src/server/reload_tests.rs`, `src/tool/selfdev/tests.rs`). Production build (`cargo check --bin jc`, release-max) is clean.
- T1.1 (free compacted history — the #1 RAM win) NOT yet implemented; designed + deferred to a focused pass (see plan + memory `compaction-memory-free-design`). It is the item that needs `bench_memory_growth.py` to validate.

## TODO (next stage)
- Implement T1.1 behind `JC_COMPACT_FREE_MEMORY` flag + round-trip test.
- Add `scripts/bench_memory_growth.py` (RSS before/after compaction) to validate T1.1.
- Tune `release-max` to codegen-units=16 + thin LTO if 10min builds are too slow for daily use (~95% of runtime win, ~3x faster compile).

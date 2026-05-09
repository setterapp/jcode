#!/usr/bin/env python3
"""
Lightweight perf bench for jcode that runs on macOS without GNU coreutils.

Captures:
- Binary size
- Cold --help / --version timings (5 runs, median + percentiles)
- Server bootstrap: provider_init total_ms (parsed from log)
- Peak RSS of a brief headless serve+exit cycle (via /usr/bin/time -l)
- Build info / commit hash for traceability

Outputs JSON to stdout (or --out path).
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import socket
import statistics
import subprocess
import sys
import tempfile
import time
from dataclasses import asdict, dataclass
from pathlib import Path


@dataclass
class CmdTiming:
    runs: int
    min_ms: float
    median_ms: float
    p95_ms: float
    max_ms: float
    mean_ms: float
    stdev_ms: float


def time_cmd(args: list[str], runs: int = 5) -> CmdTiming:
    samples = []
    for _ in range(runs):
        t0 = time.perf_counter()
        subprocess.run(args, capture_output=True, check=False)
        samples.append((time.perf_counter() - t0) * 1000.0)
    samples.sort()
    n = len(samples)
    p95 = samples[max(0, int(n * 0.95) - 1)]
    return CmdTiming(
        runs=runs,
        min_ms=round(samples[0], 3),
        median_ms=round(statistics.median(samples), 3),
        p95_ms=round(p95, 3),
        max_ms=round(samples[-1], 3),
        mean_ms=round(statistics.mean(samples), 3),
        stdev_ms=round(statistics.stdev(samples) if n > 1 else 0.0, 3),
    )


def measure_peak_rss(binary: Path) -> dict:
    """Spawn jcode serve with user's real config but isolated runtime dir.
    Caller must NOT have a jcode daemon running on the system socket — but we
    use a private runtime dir so we don't conflict either way."""
    tmp_runtime = tempfile.mkdtemp(prefix="jcode-bench-rt-")
    env = os.environ.copy()
    env["JCODE_RUNTIME_DIR"] = tmp_runtime
    env["JCODE_NO_TELEMETRY"] = "1"

    # /usr/bin/time -l prints peak resident set size (bytes on macOS) to stderr
    # at process exit. We boot the server, wait for the socket, then SIGTERM.
    socket_path = Path(tmp_runtime) / "jcode.sock"

    # We can't easily measure peak RSS of a process we kill with SIGTERM since
    # /usr/bin/time waits for clean exit. Instead use ps -o rss after the
    # process is "alive and idle" — captures steady-state RSS, not peak,
    # which is the more interesting number for daily use anyway.
    proc = subprocess.Popen(
        [str(binary), "--provider", "auto", "serve"],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    try:
        # Wait up to 10s for socket to appear
        deadline = time.time() + 10.0
        while time.time() < deadline:
            if socket_path.exists():
                break
            time.sleep(0.05)
        else:
            proc.kill()
            return {"error": "server failed to bind socket within 10s"}

        # Let it settle
        time.sleep(2.0)

        # Probe RSS via ps
        rss_kb = int(
            subprocess.check_output(
                ["ps", "-o", "rss=", "-p", str(proc.pid)],
                stderr=subprocess.DEVNULL,
            )
            .decode()
            .strip()
        )
        # Read provider_init time from today's log
        provider_init_ms = None
        log_path = Path.home() / ".jcode" / "logs"
        if log_path.exists():
            today = time.strftime("%Y-%m-%d")
            log_file = log_path / f"jcode-{today}.log"
            if log_file.exists():
                # Read tail to avoid huge files
                with open(log_file, "rb") as f:
                    f.seek(0, 2)
                    sz = f.tell()
                    f.seek(max(0, sz - 200_000), 0)
                    tail = f.read().decode(errors="ignore")
                m = re.findall(r"provider_init:.*total=(\d+)ms", tail)
                if m:
                    provider_init_ms = int(m[-1])

        return {
            "rss_idle_kb": rss_kb,
            "rss_idle_mb": round(rss_kb / 1024.0, 2),
            "provider_init_ms": provider_init_ms,
        }
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=3.0)
        except subprocess.TimeoutExpired:
            proc.kill()
        shutil.rmtree(tmp_runtime, ignore_errors=True)


def get_binary_info(binary: Path) -> dict:
    size = binary.stat().st_size
    return {
        "size_bytes": size,
        "size_mb": round(size / (1024 * 1024), 2),
    }


def get_git_info(repo: Path) -> dict:
    def git(args: list[str]) -> str:
        return subprocess.check_output(["git", *args], cwd=repo).decode().strip()

    return {
        "commit": git(["rev-parse", "HEAD"]),
        "short_commit": git(["rev-parse", "--short", "HEAD"]),
        "branch": git(["rev-parse", "--abbrev-ref", "HEAD"]),
        "dirty": bool(git(["status", "--porcelain"])),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", nargs="?", default="./target/release/jcode")
    parser.add_argument("--out", default="-", help="JSON output path or - for stdout")
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--label", default="run")
    args = parser.parse_args()

    binary = Path(args.binary).resolve()
    if not binary.exists():
        print(f"binary not found: {binary}", file=sys.stderr)
        return 1

    repo_root = Path(__file__).resolve().parent.parent

    print("Measuring --help...", file=sys.stderr)
    help_t = time_cmd([str(binary), "--help"], runs=args.runs)

    print("Measuring --version...", file=sys.stderr)
    version_t = time_cmd([str(binary), "--version"], runs=args.runs)

    print("Measuring server idle RSS...", file=sys.stderr)
    rss = measure_peak_rss(binary)

    out = {
        "label": args.label,
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S %z"),
        "binary": str(binary),
        "binary_info": get_binary_info(binary),
        "git": get_git_info(repo_root),
        "platform": {
            "os": sys.platform,
            "arch": os.uname().machine,
        },
        "help_timing_ms": asdict(help_t),
        "version_timing_ms": asdict(version_t),
        "server_idle": rss,
    }

    out_str = json.dumps(out, indent=2)
    if args.out == "-":
        print(out_str)
    else:
        Path(args.out).write_text(out_str + "\n")
        print(f"Wrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())

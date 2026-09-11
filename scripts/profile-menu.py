#!/usr/bin/env python3
"""Profile the real macOS app using only Python stdlib and Apple system tools."""
import argparse
import json
import os
import platform
import signal
import subprocess
import tempfile
import time
from pathlib import Path


def cpu_seconds(value):
    parts = value.split(":")
    return sum(float(part) * 60 ** index for index, part in enumerate(reversed(parts)))


def usage(pid):
    result = subprocess.run(
        ["ps", "-p", str(pid), "-o", "time=", "-o", "rss="],
        capture_output=True, text=True, check=False,
    )
    fields = result.stdout.split()
    if len(fields) != 2:
        return None
    return {"monotonic": time.monotonic(), "cpu_seconds": cpu_seconds(fields[0]),
            "rss_mib": int(fields[1]) / 1024}


def find_app(executable, directory):
    candidates = subprocess.run(["pgrep", "-x", "rimv-menu-bar"], capture_output=True, text=True, check=False)
    for candidate in candidates.stdout.split():
        command = subprocess.run(["ps", "-p", candidate, "-o", "command="],
                                 capture_output=True, text=True, check=False).stdout
        if str(executable) in command and str(directory) in command:
            return int(candidate)
    return None


def stop_app(executable, directory, launcher):
    pid = find_app(executable, directory)
    if pid is not None:
        try:
            os.kill(pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    try:
        launcher.wait(timeout=20)
    except subprocess.TimeoutExpired:
        # Re-resolve the unique recording path before killing a timed-out app.
        pid = find_app(executable, directory)
        if pid is not None:
            try:
                os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        launcher.terminate()
        launcher.wait(timeout=5)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, required=True)
    parser.add_argument("--mode", choices=["idle", "mic", "both", "toggles"], default="idle")
    parser.add_argument("--seconds", type=int, default=30)
    parser.add_argument("--max-cpu-percent", type=float, help="optional steady CPU regression budget")
    parser.add_argument("--max-rss-mib", type=float, help="optional peak resident memory budget")
    args = parser.parse_args()
    if platform.system() != "Darwin":
        parser.error("this profiler requires macOS")
    if not 15 <= args.seconds <= 3600:
        parser.error("use 15–3600 seconds; the first five seconds are warm-up")
    if any(value is not None and (not 0 < value < float("inf"))
           for value in [args.max_cpu_percent, args.max_rss_mib]):
        parser.error("resource budgets must be positive finite numbers")
    # Avoid measuring concurrent copies or recording without obvious ownership.
    existing = subprocess.run(["pgrep", "-x", "rimv-menu-bar"], capture_output=True, check=False)
    if existing.returncode == 0:
        parser.error("quit the existing rimv menu app before profiling")
    executable = args.app.resolve() / "Contents/MacOS/rimv-menu-bar"
    if not executable.is_file():
        parser.error(f"build the app first: {executable}")
    root = Path("target/profiles")
    root.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix=f"{args.mode}-", dir=root)).resolve()
    print(f"Profiling {args.mode} for {args.seconds}s; results: {output}", flush=True)
    # LaunchServices gives TCC the same app identity as a normal Finder launch.
    # Directly spawning the executable can attribute permission to the terminal.
    events_path = output / "events.log"
    events_path.touch()
    command = ["open", "-n", "-W", "--stdout", str(events_path),
               "--stderr", str(output / "stderr.log"), str(args.app.resolve()), "--args",
               "--profile", args.mode, "--seconds", str(args.seconds),
               "--recordings", str(output / "recordings")]
    rows = []
    ready_at, ready_snapshot, final = None, None, None
    stack = None
    timed_out = False
    started = time.monotonic()
    next_usage = started
    app_pid = None
    with (output / "tools.log").open("w") as errors, events_path.open() as events:
        process = subprocess.Popen(command, stdout=errors, stderr=errors)
        try:
            while True:
                finished = process.poll() is not None
                if app_pid is None:
                    app_pid = find_app(executable, output / "recordings")
                while line := events.readline():
                    if line.startswith("PROFILE_READY "):
                        ready_at = time.monotonic()
                        ready_snapshot = json.loads(line.removeprefix("PROFILE_READY "))
                        # Sample during warm-up; exclude its overhead from CPU statistics.
                        if app_pid is not None:
                            stack = subprocess.Popen(
                                ["sample", str(app_pid), "3", "10", "-file", str(output / "stacks.txt")],
                                stdout=subprocess.DEVNULL, stderr=errors,
                            )
                    elif line.startswith("PROFILE_RESULT "):
                        final = json.loads(line.removeprefix("PROFILE_RESULT "))
                now = time.monotonic()
                if now >= next_usage and process.poll() is None and app_pid is not None:
                    row = usage(app_pid)
                    if row:
                        row["since_launch_seconds"] = row["monotonic"] - started
                        row["measured"] = (ready_at is not None and now >= ready_at + 5
                                           and (stack is None or stack.poll() is not None)
                                           and final is None)
                        rows.append(row)
                    next_usage = now + 1
                if finished:
                    break
                if now - started > args.seconds + 120:
                    timed_out = True
                    stop_app(executable, output / "recordings", process)
                    break
                time.sleep(0.2)
        except KeyboardInterrupt:
            stop_app(executable, output / "recordings", process)
            raise
        finally:
            # A parser/tool failure must not leave a hidden recording running.
            if process.poll() is None:
                stop_app(executable, output / "recordings", process)
        if stack is not None:
            stack.wait(timeout=10)
    measured = [row for row in rows if row["measured"]]
    cpu = None
    if len(measured) >= 2:
        cpu = 100 * (measured[-1]["cpu_seconds"] - measured[0]["cpu_seconds"]) / (
            measured[-1]["monotonic"] - measured[0]["monotonic"])
    metadata = []
    for path in (output / "recordings").glob("*/session.json"):
        metadata.append(json.loads(path.read_text()))
    expected = [] if args.mode == "idle" else ["microphone"]
    if args.mode == "both":
        expected.append("system")
    recorded = {recording["source"] for session in metadata for recording in session["recordings"]
                if recording["received_sample_frames"] > 0}
    drops = (final or {}).get("snapshot", {})
    passed = (process.returncode == 0 and final is not None and final["error"] is None
              and not timed_out and cpu is not None and set(expected) <= recorded
              and drops.get("dropped_microphone_blocks", 0) == 0
              and drops.get("dropped_system_blocks", 0) == 0
              and drops.get("last_error") is None)
    peak_rss = max((row["rss_mib"] for row in rows), default=None)
    budget_passed = ((args.max_cpu_percent is None or (cpu is not None and cpu <= args.max_cpu_percent))
                     and (args.max_rss_mib is None or (peak_rss is not None and peak_rss <= args.max_rss_mib)))
    report = {
        "mode": args.mode, "requested_seconds": args.seconds,
        "platform": platform.platform(), "architecture": platform.machine(),
        "executable_bytes": executable.stat().st_size,
        "measurement": "ps cumulative CPU delta / wall time; 100% is one CPU core; RSS is resident memory, not physical footprint",
        "warmup_seconds": 5, "measured_samples": len(measured),
        "average_cpu_percent_one_core": cpu,
        "peak_rss_mib": peak_rss,
        "steady_rss_first_mib": measured[0]["rss_mib"] if measured else None,
        "steady_rss_last_mib": measured[-1]["rss_mib"] if measured else None,
        "stack_sample_exit_code": stack.returncode if stack else None,
        "launcher_exit_code": process.returncode, "timed_out": timed_out,
        "launch_method": "macOS LaunchServices (open -n -W); native exit code is unavailable; final app result and metadata are required",
        "capture_validation_passed": passed, "ready_snapshot": ready_snapshot,
        "performance_budget_passed": budget_passed,
        "budgets": {"max_cpu_percent_one_core": args.max_cpu_percent, "max_rss_mib": args.max_rss_mib},
        "result": final, "sessions": metadata, "samples": rows,
        "recording_bytes": sum(path.stat().st_size for path in (output / "recordings").rglob("*.wav")),
    }
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({key: report[key] for key in ["average_cpu_percent_one_core", "peak_rss_mib",
                     "steady_rss_first_mib", "steady_rss_last_mib", "capture_validation_passed",
                     "performance_budget_passed"]}, indent=2))
    print(f"Report: {output / 'report.json'}\nStacks: {output / 'stacks.txt'}")
    return 0 if passed and budget_passed else 1


if __name__ == "__main__":
    raise SystemExit(main())

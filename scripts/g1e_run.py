#!/usr/bin/env python3
"""One fixed-pool timing campaign plus 45 isolated operation-memory processes."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import shutil
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--raw", default="experiments/g1_e_raw")
    args = parser.parse_args()
    out = Path(args.raw)
    if out.exists():
        raise SystemExit("refusing to overwrite existing G1-E campaign")
    binary = str(Path("target/release/friendly-g1e-fixed-suite-benchmark").resolve())
    env = os.environ.copy()
    log_dir = Path(tempfile.mkdtemp(prefix="verivoting-g1e-logs-"))
    with (log_dir / "timing.log").open("w") as log:
        process = subprocess.Popen([binary, "--output", str(out)], env=env,
                                   stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        for line in process.stdout:
            print(line, end="", flush=True)
            log.write(line)
            log.flush()
        if process.wait():
            raise SystemExit(f"timing failed; retained log at {log_dir}")
    shutil.copy2(log_dir / "timing.log", out / "timing.log")
    jobs = [(b, op, repeat) for b in [2, 4, 8, 16, 32]
            for op in range(3) for repeat in range(1, 4)]
    random.Random(20261030).shuffle(jobs)
    for index, (batch, op, repeat) in enumerate(jobs, 1):
        target = out / "memory" / f"B{batch}-op{op}-repeat{repeat}.json"
        subprocess.run([binary, "--memory", str(batch), str(op), str(repeat), str(target)],
                       env=env, check=True)
        print(f"memory {index}/45: B={batch} op={op} repeat={repeat} PASS", flush=True)
    manifest = json.loads((out / "manifest.json").read_text())
    for item in manifest["source_hashes"]:
        assert hashlib.sha256(Path(item["path"]).read_bytes()).hexdigest() == item["sha256"]
    assert subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip() == manifest["git_commit"]
    assert not subprocess.check_output(["git", "diff", "HEAD", "--name-only"], text=True).strip()
    print("G1-E measurement campaign complete: timing150 / operation windows450 / memory45")


if __name__ == "__main__":
    main()

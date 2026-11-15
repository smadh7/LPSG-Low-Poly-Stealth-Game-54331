#!/usr/bin/env python3
"""Run the game the way the shipped build does, for local verification.

Bevy resolves assets next to the executable unless told otherwise, so a debug
binary in `target/debug` cannot see `assets/` by itself. This script sets the
environment the packaged build relies on and runs the game from an arbitrary
working directory — which is exactly the situation Section 11 describes.

    tools/local_run.py                 # normal play
    tools/local_run.py --smoke 120     # boot, run 120 frames, exit
    tools/local_run.py --capture "a:0,2,-30,0,2,0;b:40,2,-10,30,1,-26"
"""
import argparse
import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--smoke", type=int, metavar="FRAMES")
    parser.add_argument("--capture", metavar="SPEC")
    parser.add_argument("--exe", default=None)
    args = parser.parse_args()

    exe = pathlib.Path(args.exe) if args.exe else ROOT / "target" / "debug" / "hitman_lowpoly"
    if not exe.exists():
        exe = exe.with_suffix(".exe")
    if not exe.exists():
        print(f"build first: cargo build --bin hitman_lowpoly\nmissing {exe}", file=sys.stderr)
        return 2

    env = dict(os.environ, HITMAN_ASSET_ROOT=str(ROOT / "assets"), RUST_LOG="info,wgpu=warn")
    if args.smoke:
        env["HITMAN_SMOKE_FRAMES"] = str(args.smoke)
    if args.capture:
        env["HITMAN_CAPTURE"] = args.capture
        (ROOT / "captures").mkdir(exist_ok=True)
    # Deliberately run from the repository root rather than the executable's
    # folder: the asset locator must not depend on the working directory.
    return subprocess.call([str(exe)], cwd=ROOT, env=env)


if __name__ == "__main__":
    raise SystemExit(main())

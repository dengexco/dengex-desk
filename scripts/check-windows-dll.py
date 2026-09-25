#!/usr/bin/env python3
"""Bounded native DLL load check with non-sensitive loader diagnostics."""
from pathlib import Path
import subprocess
import sys

exe = Path(sys.argv[1]).resolve()
try:
    result = subprocess.run([str(exe), "--load-check"], capture_output=True, timeout=30)
except subprocess.TimeoutExpired as error:
    print("DLL load check timed out", flush=True)
    print((error.stdout or b"").decode(errors="replace"), flush=True)
    print((error.stderr or b"").decode(errors="replace"), flush=True)
    raise SystemExit(1)
print(result.stdout.decode(errors="replace"), flush=True)
print(result.stderr.decode(errors="replace"), flush=True)
if result.returncode:
    raise SystemExit(f"DLL load check failed: {result.returncode}")
print("Real application DLL load and ABI verification passed.")

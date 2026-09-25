#!/usr/bin/env python3
"""Native Windows negative tests. Fixtures never capture a screen or use network."""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("launcher_build", ROOT / "scripts/build-windows-launcher.py")
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)


def invoke(exe, cwd, flag, expected):
    result = subprocess.run([str(exe), flag], cwd=cwd, timeout=20)
    if result.returncode != expected:
        raise AssertionError(f"{flag}: expected {expected}, received {result.returncode}")


def main():
    if os.name != "nt":
        raise SystemExit("Requires native Windows and an MSVC developer environment")
    with tempfile.TemporaryDirectory(prefix="dengeX DLL Türkçe ") as directory:
        root = Path(directory)
        package = root / "Paket boşluk"
        unrelated = root / "untrusted-cwd"
        package.mkdir()
        unrelated.mkdir()
        source = root / "fixture.c"
        source.write_text('#include <windows.h>\n__declspec(dllexport) DWORD WINAPI DxAppAbiVersion(void){return 1;}\n__declspec(dllexport) DWORD WINAPI DxAppRun(void){return 0;}\n')
        dll = package / "dengex_desktop.dll"
        subprocess.run(["cl", "/nologo", "/LD", "/MT", str(source), f"/Fe{dll}", f"/Fo{root / 'fixture.obj'}"], check=True, cwd=root)
        good = dll.read_bytes()
        exe = package / "dengeX Remote.exe"
        builder.build(dll, exe)
        invoke(exe, unrelated, "--verify-package", 0)
        invoke(exe, unrelated, "--load-check", 0)
        shutil.copyfile(dll, unrelated / dll.name)
        dll.unlink()
        invoke(exe, unrelated, "--load-check", 12)  # no CWD fallback
        dll.write_bytes(good[:-1] + bytes([good[-1] ^ 1]))
        invoke(exe, unrelated, "--verify-package", 13)
        invoke(exe, unrelated, "--load-check", 13)  # checked before execution
        dll.write_bytes(b"invalid PE / truncated file")
        invoke(exe, unrelated, "--load-check", 13)
        dll.write_bytes(good)
        invoke(exe, unrelated, "--load-check", 0)
    print(json.dumps({"result":"passed", "nativeWindows":True, "checks":["valid_hash", "native_dll_load", "unicode_path", "missing_dll_no_cwd_fallback", "modified_dll_blocked", "truncated_dll_blocked", "restored_dll_loads"]}))


if __name__ == "__main__":
    main()

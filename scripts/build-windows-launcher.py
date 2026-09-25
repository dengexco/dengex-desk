#!/usr/bin/env python3
"""Build a small launcher pinned to the exact DLL bytes. No signing key needed."""
import argparse
import hashlib
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def build(dll, output):
    dll, output = Path(dll).resolve(), Path(output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    hasher = hashlib.sha256()
    with dll.open("rb") as stream:
        for block in iter(lambda: stream.read(65536), b""):
            hasher.update(block)
    digest = hasher.digest()
    source = ROOT / "native/windows-launcher"
    with tempfile.TemporaryDirectory(prefix="dengex-launcher-") as directory:
        temp = Path(directory)
        (temp / "runtime_hash.h").write_text(
            "static const unsigned char DX_RUNTIME_SHA256[32] = {" +
            ",".join(f"0x{byte:02x}" for byte in digest) + "};\n"
        )
        compiler = ["cl"] if os.name == "nt" else [str(ROOT / ".tools/llvm/llvm/23.1.1/bin/clang-cl"), "--target=x86_64-pc-windows-msvc"]
        includes, libraries = [], []
        if os.name != "nt":
            sdk = ROOT / ".tools/xwin-cache/xwin"
            includes = [f"/imsvc{sdk / p}" for p in ["crt/include", "sdk/include/ucrt", "sdk/include/um", "sdk/include/shared"]]
            libraries = [f"/libpath:{sdk / p}" for p in ["crt/lib/x86_64", "sdk/lib/um/x86_64", "sdk/lib/ucrt/x86_64"]]
        compile_args = ["/nologo", "/O2", "/W4", "/WX", "/MT", "/guard:cf", "/utf-8", f"/I{temp}"] + includes + [f"/Tc{source / 'launcher.c'}", f"/Fo{temp / 'launcher.obj'}"]
        link_args = libraries + ["/SUBSYSTEM:WINDOWS", "/DYNAMICBASE", "/NXCOMPAT", "/HIGHENTROPYVA", "/guard:cf", "/MANIFEST:EMBED",
                                f"/MANIFESTINPUT:{source / 'app.manifest'}", "bcrypt.lib", "user32.lib", "kernel32.lib", "shell32.lib"]
        if os.name == "nt":
            subprocess.run(compiler + compile_args + [f"/Fe{output}", "/link"] + link_args, check=True)
        else:
            subprocess.run(compiler + compile_args + ["/c"], check=True)
            resource = temp / "app.rc"
            resource.write_text('1 24 "' + (source / 'app.manifest').as_posix() + '"\n')
            subprocess.run([str(ROOT / ".tools/llvm/llvm/23.1.1/bin/llvm-rc"), "/no-preprocess", "/FO", str(temp / "app.res"), str(resource)], check=True)
            link_args = [arg for arg in link_args if not arg.startswith("/MANIFEST")]
            link_args += ["/MANIFEST:NO", str(temp / "app.res")]
            # System Python may strip DYLD_* inherited from the shell (macOS SIP).
            env = dict(os.environ, DYLD_LIBRARY_PATH=str(ROOT / ".tools/rustup/toolchains/stable-aarch64-apple-darwin/lib"))
            subprocess.run([str(ROOT / ".tools/windows-bin/lld-link"), str(temp / "launcher.obj"), f"/out:{output}"] + link_args, check=True, env=env)
    print(f"Launcher: {output.name}; DLL SHA-256: {digest.hex()}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dll")
    parser.add_argument("output")
    args = parser.parse_args()
    build(args.dll, args.output)

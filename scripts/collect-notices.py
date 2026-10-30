#!/usr/bin/env python3
"""Bundle upstream copyright and license files from installed locked dependencies."""
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
meta = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--format-version', '1'], cwd=ROOT))
sections = ['# Third-party notices', '', 'dengeX Remote is MIT licensed. Dependencies retain their own licenses.',
            'This inventory includes build tools and dependencies for other platforms.',
            'Unmodified Rust source: https://crates.io/crates/<package>/<version> .',
            'Unmodified npm source: https://www.npmjs.com/package/<package>/v/<version> .', '']
packages = [(p['name'], p['version'], p.get('license', ''), Path(p['manifest_path']).parent)
            for p in meta['packages'] if p.get('source')]
lock = json.loads((ROOT / 'package-lock.json').read_text(encoding="utf-8"))
for key, entry in lock['packages'].items():
    if 'node_modules/' not in key or entry.get('link'):
        continue
    directory = ROOT / key
    if not directory.is_dir():
        continue
    manifest = directory / 'package.json'
    data = json.loads(manifest.read_text(encoding="utf-8")) if manifest.exists() else {}
    packages.append((data.get('name', key), entry.get('version', ''), data.get('license', ''), directory))
for name, version, license_name, directory in sorted(packages, key=lambda p: (p[0], p[1])):
    sections += [f'## {name} {version}', '', f'Declared license: {license_name}', '']
    files = sorted({p for pattern in ['LICENSE*', 'LICENCE*', 'COPYING*', 'NOTICE*', 'license*', 'licence*']
                    for p in directory.glob(pattern) if p.is_file()})
    for path in files:
        sections += [f'### {path.name}', '', '```text', path.read_text(encoding="utf-8", errors='replace'), '```', '']
    if not files:
        sections += ['See the unmodified package source for its license and copyright notices.', '']
output = ROOT / '.artifacts/THIRD-PARTY-NOTICES.txt'
output.parent.mkdir(exist_ok=True)
output.write_text('\n'.join(sections), encoding='utf-8')
print(f'Collected notices for {len(packages)} dependency packages')

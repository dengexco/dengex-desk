#!/usr/bin/env python3
"""Export CycloneDX component inventory from actual locked Cargo metadata.
Not a legal clearance report. npm's native SBOM command covers JavaScript.
"""
import json, uuid, datetime
from pathlib import Path
root=Path(__file__).resolve().parent.parent
m=json.loads((root/'.artifacts/cargo-metadata.json').read_text())
components=[]
refs={}
for p in m['packages']:
    name,version=p['name'],p['version']
    ref=f'pkg:cargo/{name}@{version}'
    refs[p['id']]=ref
    c={'type':'library','bom-ref':ref,'name':name,'version':version,'purl':ref}
    if p.get('license'): c['licenses']=[{'expression':p['license']}]
    components.append(c)
deps=[]
for n in (m.get('resolve') or {}).get('nodes',[]):
    deps.append({'ref':refs[n['id']],'dependsOn':sorted(set(refs[i['pkg']] for i in n['deps']))})
out={'bomFormat':'CycloneDX','specVersion':'1.5','serialNumber':'urn:uuid:'+str(uuid.uuid4()),'version':1,
     'metadata':{'timestamp':datetime.datetime.now(datetime.timezone.utc).isoformat()},'components':components,'dependencies':deps}
(root/'.artifacts/sbom-rust.json').write_text(json.dumps(out,indent=2))
lines=['# Rust dependency license inventory','','Generated from Cargo.lock-resolved cargo metadata. Includes target-specific dependencies, not all of which ship on macOS/Windows. License expressions are upstream metadata, not legal approval.','','| Package | Version | Declared license |','|---|---|---|']
for p in sorted(m['packages'],key=lambda p:p['name']):
    lines.append('| '+p['name']+' | '+p['version']+' | '+(p.get('license') or 'Not declared / local source')+' |')
(root/'docs/dependency-licenses.md').write_text('\n'.join(lines)+'\n')
print(f'Exported {len(components)} Rust dependency components')

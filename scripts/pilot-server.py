#!/usr/bin/env python3
"""Project-local pilot lifecycle. No boot persistence and no existing tunnel changes."""
import os,sys,json,pathlib,secrets,subprocess,time,re,urllib.request,signal
root=pathlib.Path(__file__).resolve().parents[1];work=root/'.artifacts/pilot';work.mkdir(parents=True,exist_ok=True)
statefile=work/'processes.json'
def alive(pid,binary):
 try:
  command=subprocess.check_output(['ps','-p',str(pid),'-o','command='],text=True).strip()
  return command.startswith(str(binary)+' ') or command==str(binary)
 except subprocess.CalledProcessError:return False
mode=sys.argv[1] if len(sys.argv)>1 else 'status'
if mode=='stop':
 if statefile.exists():
  s=json.loads(statefile.read_text())
  for name in ['tunnel','api']:
   if alive(s[name]['pid'],s[name]['binary']):os.kill(s[name]['pid'],signal.SIGTERM)
  statefile.unlink()
 print('Pilot processes stopped.');sys.exit()
if mode=='status':
 if not statefile.exists():print('Pilot is stopped.');sys.exit()
 s=json.loads(statefile.read_text());print(json.dumps({'url':s['url'],'apiRunning':alive(s['api']['pid'],s['api']['binary']),'tunnelRunning':alive(s['tunnel']['pid'],s['tunnel']['binary'])},indent=2));sys.exit()
if mode!='start':raise SystemExit('Use start, status, or stop')
if statefile.exists():raise SystemExit('Pilot state exists. Run status or stop first.')
api=root/'.artifacts/pilot/dx-pilot-api';tunnel=root/'.tools/cloudflared/cloudflared'
tokenfile=work/'create-token'
if not tokenfile.exists():
 fd=os.open(tokenfile,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
 with os.fdopen(fd,'w') as f:f.write(secrets.token_urlsafe(32))
env=dict(os.environ,DX_LISTEN='127.0.0.1:18433',DX_PILOT_GUEST_HOSTS='1',DX_PILOT_TOKEN_FILE=str(tokenfile),DX_PILOT_WINDOWS_ZIP=str(root/'.artifacts/releases/dengeX-Remote-Windows-x64.zip'))
p=subprocess.Popen([str(api)],env=env,stdout=open(work/'api.log','ab'),stderr=subprocess.STDOUT,start_new_session=True)
q=None
try:
 for _ in range(40):
  if p.poll() is not None:raise RuntimeError('API failed to start; inspect .artifacts/pilot/api.log')
  try:
   with urllib.request.urlopen('http://127.0.0.1:18433/healthz',timeout=1) as r:
    if r.status==200:break
  except Exception:time.sleep(.25)
 log=work/'tunnel.log'
 q=subprocess.Popen([str(tunnel),'tunnel','--no-autoupdate','--url','http://127.0.0.1:18433'],stdout=open(log,'wb'),stderr=subprocess.STDOUT,start_new_session=True)
 url=None
 for _ in range(120):
  if q.poll() is not None:raise RuntimeError('Tunnel failed to start; inspect .artifacts/pilot/tunnel.log')
  m=re.search(r'https://[a-z0-9-]+\.trycloudflare\.com',log.read_text(errors='replace'))
  if m:url=m.group(0);break
  time.sleep(.5)
 if not url:raise RuntimeError('Tunnel URL timeout')
 data={'url':url,'api':{'pid':p.pid,'binary':str(api)},'tunnel':{'pid':q.pid,'binary':str(tunnel)}}
 statefile.write_text(json.dumps(data,indent=2))
 public=root/'apps/desktop/src-tauri/resources/pilot-server.json';public.parent.mkdir(parents=True,exist_ok=True);public.write_text(json.dumps({'serverUrl':url}))
 local=pathlib.Path.home()/'Library/Application Support/com.dengex.remote.lab/pilot-host.json';local.parent.mkdir(parents=True,exist_ok=True)
 fd=os.open(local,os.O_WRONLY|os.O_CREAT|os.O_TRUNC,0o600)
 with os.fdopen(fd,'w') as f:json.dump({'serverUrl':url,'createToken':tokenfile.read_text().strip()},f)
 print(url)
except Exception:
 p.terminate()
 if q:q.terminate()
 raise

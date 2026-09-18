#!/usr/bin/env python3
import json,pathlib,urllib.request,urllib.error
root=pathlib.Path(__file__).resolve().parents[1]
state=json.loads((root/'.artifacts/pilot/processes.json').read_text());url=state['url'];admin=(root/'.artifacts/pilot/create-token').read_text().strip()
def request(path,method='GET',token='',body=None):
 data=None if body is None else json.dumps(body).encode();headers={'Content-Type':'application/json'}
 if token:headers['Authorization']='Bearer '+token
 try:
  with urllib.request.urlopen(urllib.request.Request(url+path,data=data,headers=headers,method=method),timeout=20) as r:return r.status,json.load(r)
 except urllib.error.HTTPError as e:return e.code,json.load(e)
assert request('/healthz')[0]==200
assert request('/pilot/create','POST','bad',{})[0]==401
code,a=request('/pilot/create','POST',admin,{})
assert code==201
code,b=request('/pilot/join','POST',body={'invite':a['invite'],'name':'HTTPS doğrulama cihazı'})
assert code==200
assert request('/pilot/join','POST',body={'invite':a['invite'],'name':'Yeniden kullanma'})[0]==401
assert request('/pilot/action','POST',b['token'],{'action':'accept'})[0]==409
assert request('/pilot/action','POST',a['token'],{'action':'accept'})[0]==200
assert request('/pilot/action','POST',a['token'],{'action':'end'})[0]==200
assert request('/pilot/room',token=b['token'])[1]['state']=='ended'
code,guest=request('/pilot/create-guest','POST',body={})
assert code==201
assert request('/pilot/create','POST',guest['token'],{})[0]==401
code,mac=request('/pilot/join','POST',body={'invite':guest['invite'],'name':'Mac doğrulama alıcısı'})
assert code==200
assert request('/pilot/action','POST',mac['token'],{'action':'accept'})[0]==409
assert request('/pilot/action','POST',guest['token'],{'action':'accept'})[0]==200
assert request('/pilot/action','POST',guest['token'],{'action':'end'})[0]==200
result={'result':'passed','url':'https://pilot.example.invalid','addressRedacted':True,'tests':['public_https_health','unauthenticated_admin_creation_rejected','single_use_invitation','viewer_cannot_consent','host_consent','revocation','guest_host_creation','guest_cannot_administer','guest_host_local_consent'],'mediaTest':False,'twoDeviceTest':False}
(root/'docs/testing/evidence/cloudflare-api.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))

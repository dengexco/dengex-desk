#!/usr/bin/env python3
"""Private localhost-only TURN transport fixture. NOT an Internet TURN config.
Deliberately permits loopback peers inside this isolated, authenticated fixture.
Production config continues to deny private/loopback peer destinations.
"""
import base64, hashlib, hmac, json, os, pathlib, secrets, socket, subprocess, time, sys, tempfile
root=pathlib.Path(__file__).resolve().parent.parent
art=root/'.artifacts'
art.mkdir(exist_ok=True)
secret=secrets.token_urlsafe(48)
private=tempfile.TemporaryDirectory(prefix='turn-fixture-',dir=art)
config=pathlib.Path(private.name)/'turn-loopback.conf'
linux=sys.platform.startswith('linux')
config.write_text('\n'.join(['listening-port='+('34780' if linux else '3478'),'listening-ip='+('127.0.0.1' if linux else '0.0.0.0'),'relay-ip=127.0.0.1','external-ip=127.0.0.1','min-port=51000','max-port=51020','realm=dengex-local.invalid','fingerprint','use-auth-secret','static-auth-secret='+secret,'no-tls','no-dtls','no-cli','allow-loopback-peers','no-multicast-peers','pidfile=/tmp/turnserver.pid','user-quota=8','total-quota=24','max-bps=2000000','log-file=stdout']))
config.chmod(0o444)
name='dx-phase0-relay-'+str(os.getpid())
container=None
try:
    with (art/'turn-container.log').open('w') as log:
        network=['--network','host'] if linux else ['-p','127.0.0.1:34780:3478/udp','-p','127.0.0.1:34780:3478/tcp','-p','127.0.0.1:51000-51020:51000-51020/udp']
        p=subprocess.run(['docker','run','-d','--name',name,'--cap-drop=ALL','--cap-add=NET_BIND_SERVICE','--security-opt=no-new-privileges','--read-only','--tmpfs','/tmp']+network+[
            '-v',str(config)+':/etc/coturn/turnserver.conf:ro',
            'coturn/coturn:4.15.0','-c','/etc/coturn/turnserver.conf'],stdout=subprocess.PIPE,stderr=log,text=True,timeout=90)
    if p.returncode:raise RuntimeError('TURN container could not start; see turn-container.log')
    container=p.stdout.strip()
    for _ in range(40):
        try:
            socket.create_connection(('127.0.0.1',34780),timeout=.5).close();break
        except OSError:time.sleep(.25)
    else:
        raise RuntimeError('TURN listener did not start')
    user=str(int(time.time())+240)+':phase0'
    credential=base64.b64encode(hmac.new(secret.encode(),user.encode(),hashlib.sha1).digest()).decode()
    for protocol in ['udp','tcp']:
        env=dict(os.environ,DX_TURN_URL=f'turn:127.0.0.1:34780?transport={protocol}',DX_TURN_USERNAME=user,DX_TURN_CREDENTIAL=credential)
        with (art/f'turn-{protocol}.log').open('w') as log:
            result=subprocess.run([str(root/'target/debug/dx-probe'),'transport','--relay','--report',str(art/f'turn-{protocol}.json')],env=env,stdout=log,stderr=log,timeout=60)
        print(protocol,'passed' if result.returncode==0 else 'failed',flush=True)
        if protocol == 'udp' and result.returncode:
            print((art/'turn-udp.json').read_text(),flush=True)
            raise RuntimeError('Required TURN/UDP data-channel test failed')
        if protocol == 'udp':
            subprocess.run(['cargo','test','--locked','-p','dengex-transport','--lib','remote_host_gathers_only_relay_candidates','--','--ignored'],env=env,check=True,timeout=120)
        if protocol == 'tcp' and result.returncode == 0:
            raise RuntimeError('Unsupported TCP unexpectedly fell back to a direct connection')
        if protocol=='udp' and result.returncode==0 and '--video' in sys.argv:
            video_user=str(int(time.time())+120)+':phase0-video'
            env['DX_TURN_USERNAME']=video_user
            env['DX_TURN_CREDENTIAL']=base64.b64encode(hmac.new(secret.encode(),video_user.encode(),hashlib.sha1).digest()).decode()
            with (art/'turn-native-video.log').open('w') as log:
                video=subprocess.run([str(root/'target/debug/dx-probe'),'native','--relay','--helper',str(art/'native/dx-macos-probe'),'--seconds','8','--start-local-test','--report',str(art/'turn-native-video.json')],env=env,stdout=log,stderr=log,timeout=60)
            print('native-video', 'passed' if video.returncode==0 else 'failed',flush=True)
except Exception:
    if container:
        diagnostic=subprocess.run(['docker','logs',container],capture_output=True,text=True,timeout=5)
        print((diagnostic.stdout+diagnostic.stderr).replace(secret,'[redacted]')[-6000:],flush=True)
    raise
finally:
    if container:
        with (art/'turn-server-runtime.log').open('w') as log:
            subprocess.run(['docker','logs',container],stdout=log,stderr=log,timeout=5)
        subprocess.run(['docker','rm','-f','-v',container],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=20)
    config.unlink(missing_ok=True)
    private.cleanup()

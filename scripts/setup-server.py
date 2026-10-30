#!/usr/bin/env python3
"""Initialize a Linux self-host deployment. Never overwrite existing secrets."""
import argparse
import ipaddress
from pathlib import Path
import re
import secrets

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--domain', required=True, help='Public HTTPS hostname, e.g. support.example.com')
    parser.add_argument('--turn-ip', help='Optional public IPv4 address of this Linux server')
    args = parser.parse_args()
    domain = args.domain.lower()
    if len(domain) > 253 or not re.fullmatch(r'(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}', domain):
        parser.error('--domain must be a public DNS hostname without a URL path')
    address = None
    if args.turn_ip:
        try:
            address = ipaddress.IPv4Address(args.turn_ip)
        except ValueError:
            parser.error('--turn-ip must be a public IPv4 address')
        if not address.is_global:
            parser.error('--turn-ip must be publicly routable')
    target = ROOT / 'infra/self-host'
    directory = target / 'secrets'
    if (target / '.env').exists() or directory.exists():
        parser.error('Configuration already exists. Edit it explicitly; secrets were not changed.')
    directory.mkdir(mode=0o700)
    secret = secrets.token_urlsafe(48)
    for name, value in [('admin-token', secrets.token_urlsafe(32)), ('turn-secret', secret if address else '')]:
        path = directory / name
        path.write_text(value + '\n')
        # Parent directory is owner-only; container can read the mounted file.
        path.chmod(0o444)
    config = (ROOT / 'infra/turnserver.conf').read_text()
    config = config.replace('realm=dengex-lab.invalid', 'realm=' + domain)
    config += '\nno-tcp\nno-tcp-relay\npidfile=/tmp/turnserver.pid\n'
    if address:
        config += f'listening-ip=0.0.0.0\nexternal-ip={address}\nstatic-auth-secret={secret}\n'
    path = directory / 'turnserver.conf'
    path.write_text(config)
    path.chmod(0o444)
    env = target / '.env'
    env.write_text(f'DX_DOMAIN={domain}\nDX_PILOT_GUEST_HOSTS=1\nDX_RELAY_ONLY=0\n'
                   + (f'DX_TURN_URL=turn:{address}:3478?transport=udp\nDX_TURN_SECRET_FILE=/run/secrets/turn-secret\n' if address else 'DX_TURN_URL=\nDX_TURN_SECRET_FILE=\n'))
    env.chmod(0o600)
    profile = ' --profile turn' if address else ''
    print('Configuration created. No secrets printed. Run:')
    print(f'docker compose --env-file infra/self-host/.env -f infra/self-host/compose.yaml{profile} up -d --build --wait')
    print(f'Use https://{domain} on both desktop clients.')


if __name__ == '__main__':
    main()

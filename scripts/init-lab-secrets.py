#!/usr/bin/env python3
"""Create unique lab secrets with mode 0600. Never print secret contents."""
from pathlib import Path
import os
import secrets
root = Path(__file__).resolve().parent.parent / 'infra'
def create(path, content):
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError:
        print(f'Preserved existing {path.name}')
        return
    with os.fdopen(fd, 'w') as f:
        f.write(content)
    print(f'Created {path.name} (0600)')
create(root / 'secrets/postgres_password', secrets.token_urlsafe(32)+'\n')
create(root / 'secrets/turnserver.conf', (root / 'turnserver.conf').read_text()+'\nstatic-auth-secret='+secrets.token_urlsafe(48)+'\n')
create(root / '.env', 'KC_BOOTSTRAP_ADMIN_USERNAME=lab-admin\nKC_BOOTSTRAP_ADMIN_PASSWORD='+secrets.token_urlsafe(32)+'\n')

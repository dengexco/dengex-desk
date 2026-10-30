#!/usr/bin/env python3
"""Exercise the deployed attended API; prints no tokens, names or SDP."""
import json
import sys
import urllib.error
import urllib.request

base = sys.argv[1].rstrip('/')

def request(path, body=None, token=None, expected=200):
    headers = {'Content-Type': 'application/json'}
    if token:
        headers['Authorization'] = 'Bearer ' + token
    req = urllib.request.Request(base + path, data=None if body is None else json.dumps(body).encode(), headers=headers)
    try:
        result = urllib.request.urlopen(req, timeout=10)
    except urllib.error.HTTPError as error:
        result = error
    with result:
        assert result.status == expected, f'{path}: expected {expected}, received {result.status}'
        return json.load(result)

ready = request('/readyz')
assert ready['service'] == 'dengex-remote' and ready['ready'] and ready['guestHosts']
host = request('/pilot/create-guest', {}, expected=201)
viewer = request('/pilot/join', {'invite': host['invite'], 'name': 'Deployment test'})
request('/pilot/join', {'invite': host['invite'], 'name': 'Replay'}, expected=401)
request('/pilot/action', {'action': 'accept'}, viewer['token'], expected=409)
assert 'ice' not in request('/pilot/room', token=viewer['token'])
accepted = request('/pilot/action', {'action': 'accept'}, host['token'])
assert accepted['state'] == 'negotiating' and 'ice' in accepted
request('/pilot/action', {'action': 'end'}, host['token'])
assert request('/pilot/room', token=viewer['token'])['state'] == 'ended'
request('/pilot/room', expected=401)
print('PASS: readiness, invitation, single use, host consent, ICE gating, stop, authorization')

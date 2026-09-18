# Protocol boundary

Implemented: Rust strict input parser (4 KiB), capability vocabulary, state graph,
Ed25519 signed grant verification kernel and this schema. None are exposed through
production signaling yet. Inputs are never logged.

Native probe IPC: inherited anonymous stdin/stdout pipes only; no named socket or
listening port. Each access unit is `u32 big-endian length` then Annex B H.264;
length must be 1..4,194,304. SPS/PPS accompany source frames; IDR interval 30.
No JSON/base64 video, no React state video. The helper starts a visible native
window and requires local capture consent, except the explicit developer-only
`--start-local-test` CLI experiment. The experiment has a 60s hard capture cap.

The loopback's SDP comes from the two in-process peers, so DTLS fingerprints are
not taken from an untrusted signaling server. This is NOT a network enrollment or
technician identity design. Before external signaling, bind BOTH fingerprints,
tenant/device/user, capabilities, policy version, nonce and <=60s lease in a
verified authority grant; verify device identity and native consent independently.

State deadlines planned: requested 30s, awaiting_consent 120s, negotiating 30s,
authenticating 30s, active renewable <=60s, reconnecting 15s; ended/failed terminal.
The complete WSS, idempotency and version negotiation contract is Stage 1 work.

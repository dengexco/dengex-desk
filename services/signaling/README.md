# Signaling boundary — pending Stage 1

The Phase 0 Rust test exchanges SDP in-process. This directory is the future
Go signaling module boundary, not a working WSS signaling service. Do not expose
an unauthenticated SDP relay to make the lab look like a production product.

Integration gate: validated OIDC issuer/subject + MFA, tenant/device assignment,
verified agent identity, atomic consent, signed fingerprint-bound <=60s grants,
per-message scope enforcement, cancellation, replay protection, bounded queues.

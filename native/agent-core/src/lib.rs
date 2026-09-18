//! Fail-closed session enforcement. Wall time validates authority-signed grants;
//! monotonic elapsed milliseconds enforce a non-renewable <=60 second lease.
//! This kernel is not yet connected to production identity/signaling.
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    ViewScreen,
    ControlInput,
    ClipboardRead,
    ClipboardWrite,
    FileSend,
    FileReceive,
    RestartDevice,
    ManageUnattendedAccess,
    ManageDevices,
    ViewAudit,
}
pub type Capabilities = BTreeSet<Capability>;
pub fn intersection(scopes: &[Capabilities]) -> Capabilities {
    scopes
        .first()
        .map(|first| {
            first
                .iter()
                .filter(|c| scopes.iter().all(|s| s.contains(c)))
                .copied()
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Requested,
    AwaitingConsent,
    Negotiating,
    Authenticating,
    Active,
    Reconnecting,
    Ended,
    Failed,
}
impl SessionState {
    pub fn transition(self, next: Self) -> Result<Self, Denied> {
        use SessionState::*;
        if matches!(self, Ended | Failed) {
            return Err(Denied::State);
        }
        if matches!(next, Ended | Failed)
            || matches!(
                (self, next),
                (Requested, AwaitingConsent)
                    | (AwaitingConsent, Negotiating)
                    | (Negotiating, Authenticating)
                    | (Authenticating, Active)
                    | (Active, Reconnecting)
                    | (Reconnecting, Authenticating)
            )
        {
            Ok(next)
        } else {
            Err(Denied::State)
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Denied {
    #[error("invalid_message")]
    Message,
    #[error("invalid_signature")]
    Signature,
    #[error("scope_mismatch")]
    Scope,
    #[error("expired_or_invalid_lease")]
    Expired,
    #[error("grant_replayed_or_cache_full")]
    Replay,
    #[error("capability_denied")]
    Capability,
    #[error("invalid_state")]
    State,
    #[error("stale_input")]
    StaleInput,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub v: u8,
    pub tenant_id: String,
    pub device_id: String,
    pub user_id: String,
    pub session_id: String,
    pub jti: String,
    pub policy_version: u64,
    pub input_epoch: u64,
    pub issued_at: u64,
    pub expires_at: u64,
    pub peer_fingerprint: String,
    pub local_fingerprint: String,
    pub capabilities: Capabilities,
}
/// Raw UTF-8 JSON is signed as bytes. Do not reserialize it before verification.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedGrant {
    pub payload: String,
    pub signature: String,
}
pub struct ExpectedScope<'a> {
    pub tenant: &'a str,
    pub device: &'a str,
    pub user: &'a str,
    pub session: &'a str,
    pub policy: u64,
    pub input_epoch: u64,
    pub peer_fingerprint: &'a str,
    pub local_fingerprint: &'a str,
}
#[derive(Default)]
pub struct ReplayCache(HashMap<String, u64>);
pub struct SessionGate {
    grant: Grant,
    allowed: Capabilities,
    deadline_ms: u64,
    revoked: bool,
    epoch: u64,
    critical_sequence: u64,
    pointer_sequence: u64,
}
impl SessionGate {
    #[allow(clippy::too_many_arguments)]
    pub fn verify(
        envelope: &[u8],
        authority: &VerifyingKey,
        scope: ExpectedScope<'_>,
        policy_scopes: &[Capabilities],
        replay: &mut ReplayCache,
        wall_seconds: u64,
        monotonic_ms: u64,
    ) -> Result<Self, Denied> {
        if envelope.len() > 8192 {
            return Err(Denied::Message);
        }
        let e: SignedGrant = serde_json::from_slice(envelope).map_err(|_| Denied::Message)?;
        let payload = STANDARD.decode(e.payload).map_err(|_| Denied::Message)?;
        let sig = STANDARD
            .decode(e.signature)
            .map_err(|_| Denied::Signature)?;
        let sig = Signature::from_slice(&sig).map_err(|_| Denied::Signature)?;
        authority
            .verify_strict(&payload, &sig)
            .map_err(|_| Denied::Signature)?;
        let g: Grant = serde_json::from_slice(&payload).map_err(|_| Denied::Message)?;
        if g.v != 1
            || g.tenant_id != scope.tenant
            || g.device_id != scope.device
            || g.user_id != scope.user
            || g.session_id != scope.session
            || g.policy_version != scope.policy
            || g.input_epoch != scope.input_epoch
            || g.peer_fingerprint != scope.peer_fingerprint
            || g.local_fingerprint != scope.local_fingerprint
            || !valid_fingerprint(&g.peer_fingerprint)
            || !valid_fingerprint(&g.local_fingerprint)
            || g.jti.len() < 32
            || g.jti.len() > 128
        {
            return Err(Denied::Scope);
        }
        if g.issued_at > wall_seconds
            || g.expires_at <= wall_seconds
            || g.expires_at.saturating_sub(g.issued_at) > 60
        {
            return Err(Denied::Expired);
        }
        replay.0.retain(|_, expires| *expires > wall_seconds);
        if replay.0.contains_key(&g.jti) || replay.0.len() >= 4096 {
            return Err(Denied::Replay);
        }
        let mut scopes = policy_scopes.to_vec();
        // Missing any of customer/device/assignment/session/OS policy denies all.
        if scopes.len() != 5 {
            return Err(Denied::Scope);
        }
        scopes.push(g.capabilities.clone());
        let allowed = intersection(&scopes);
        let deadline_ms = monotonic_ms
            .checked_add((g.expires_at - wall_seconds) * 1000)
            .ok_or(Denied::Expired)?;
        replay.0.insert(g.jti.clone(), g.expires_at);
        let epoch = g.input_epoch;
        Ok(Self {
            grant: g,
            allowed,
            deadline_ms,
            revoked: false,
            epoch,
            critical_sequence: 0,
            pointer_sequence: 0,
        })
    }
    pub fn check(&self, capability: Capability, now_ms: u64) -> Result<(), Denied> {
        if self.revoked || now_ms >= self.deadline_ms {
            return Err(Denied::Expired);
        }
        if !self.allowed.contains(&capability) {
            return Err(Denied::Capability);
        }
        Ok(())
    }
    pub fn revoke(&mut self) {
        self.revoked = true;
    }
    pub fn session_id(&self) -> &str {
        &self.grant.session_id
    }
    pub fn validate_input(&mut self, packet: &InputPacket, now_ms: u64) -> Result<(), Denied> {
        self.check(Capability::ControlInput, now_ms)?;
        packet.validate()?;
        if packet.session_id != self.grant.session_id || packet.epoch != self.epoch {
            return Err(Denied::StaleInput);
        }
        let last = if matches!(packet.event, InputEvent::Pointer { .. }) {
            &mut self.pointer_sequence
        } else {
            &mut self.critical_sequence
        };
        if packet.sequence <= *last {
            return Err(Denied::StaleInput);
        }
        *last = packet.sequence;
        Ok(())
    }
    /// Disconnect always revokes the gate. Reconnection requires a new grant
    /// and native caller must release held keys/buttons before negotiation.
    pub fn disconnect(&mut self) {
        self.revoke();
    }
}
fn valid_fingerprint(s: &str) -> bool {
    s.len() == 95
        && s.split(':').count() == 32
        && s.split(':')
            .all(|p| p.len() == 2 && p.bytes().all(|c| c.is_ascii_hexdigit()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputPacket {
    pub v: u8,
    pub session_id: String,
    pub epoch: u64,
    pub sequence: u64,
    pub event: InputEvent,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputEvent {
    Pointer { x: f64, y: f64, monitor: u32 },
    Button { button: u8, down: bool },
    Key { usage: u16, down: bool },
    Text { text: String },
    Wheel { x: i16, y: i16 },
}
impl InputPacket {
    pub fn parse(bytes: &[u8]) -> Result<Self, Denied> {
        if bytes.len() > 4096 {
            return Err(Denied::Message);
        }
        let packet: Self = serde_json::from_slice(bytes).map_err(|_| Denied::Message)?;
        packet.validate()?;
        Ok(packet)
    }
    pub fn validate(&self) -> Result<(), Denied> {
        if self.v != 1
            || self.session_id.is_empty()
            || self.session_id.len() > 64
            || self.sequence == 0
        {
            return Err(Denied::Message);
        }
        let valid = match &self.event {
            InputEvent::Pointer { x, y, .. } => {
                x.is_finite() && y.is_finite() && (0.0..=1.0).contains(x) && (0.0..=1.0).contains(y)
            }
            InputEvent::Button { button, .. } => *button < 3,
            InputEvent::Key { usage, .. } => (4..=0xe7).contains(usage),
            InputEvent::Text { text } => {
                !text.is_empty() && text.len() <= 512 && !text.contains('\0')
            }
            InputEvent::Wheel { x, y } => x.unsigned_abs() <= 1200 && y.unsigned_abs() <= 1200,
        };
        if valid {
            Ok(())
        } else {
            Err(Denied::Message)
        }
    }
}

/// A filename only, never a path. Full transfer requires handle-relative no-follow
/// opens on each OS; this check alone is intentionally not a file-transfer engine.
pub fn validate_filename(name: &str) -> Result<(), Denied> {
    if name.is_empty()
        || name.len() > 240
        || name == "."
        || name == ".."
        || name.ends_with([' ', '.'])
        || name
            .chars()
            .any(|c| c.is_control() || "/\\:<>\"|?*".contains(c))
    {
        return Err(Denied::Message);
    }
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if ["CON", "PRN", "AUX", "NUL", "CLOCK$", "CONIN$", "CONOUT$"].contains(&stem.as_str())
        || (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.chars().count() == 4
            && stem
                .chars()
                .last()
                .is_some_and(|c| "123456789¹²³".contains(c))
    {
        return Err(Denied::Message);
    }
    Ok(())
}

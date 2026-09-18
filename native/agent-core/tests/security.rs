use base64::{engine::general_purpose::STANDARD, Engine};
use dengex_agent_core::*;
use ed25519_dalek::{Signer, SigningKey};
use proptest::prelude::*;
fn caps(values: &[Capability]) -> Capabilities {
    values.iter().copied().collect()
}
fn grant() -> Grant {
    Grant {
        v: 1,
        tenant_id: "tenant-a".into(),
        device_id: "device-a".into(),
        user_id: "technician-a".into(),
        session_id: "session-a".into(),
        jti: "0123456789abcdef0123456789abcdef".into(),
        policy_version: 2,
        input_epoch: 0,
        issued_at: 100,
        expires_at: 160,
        peer_fingerprint: vec!["AB"; 32].join(":"),
        local_fingerprint: vec!["CD"; 32].join(":"),
        capabilities: caps(&[Capability::ViewScreen, Capability::ControlInput]),
    }
}
fn signed(g: &Grant) -> Vec<u8> {
    let key = SigningKey::from_bytes(&[42; 32]);
    let p = serde_json::to_vec(g).unwrap();
    serde_json::to_vec(&SignedGrant {
        payload: STANDARD.encode(&p),
        signature: STANDARD.encode(key.sign(&p).to_bytes()),
    })
    .unwrap()
}
fn verify(
    g: &Grant,
    envelope: &[u8],
    tenant: &str,
    scopes: &[Capabilities],
    replay: &mut ReplayCache,
    now: u64,
) -> Result<SessionGate, Denied> {
    SessionGate::verify(
        envelope,
        &SigningKey::from_bytes(&[42; 32]).verifying_key(),
        ExpectedScope {
            tenant,
            device: &g.device_id,
            user: &g.user_id,
            session: &g.session_id,
            policy: g.policy_version,
            input_epoch: g.input_epoch,
            peer_fingerprint: &g.peer_fingerprint,
            local_fingerprint: &g.local_fingerprint,
        },
        scopes,
        replay,
        now,
        1000,
    )
}
#[test]
fn tenant_scope_cannot_be_swapped() {
    let g = grant();
    assert_eq!(
        verify(
            &g,
            &signed(&g),
            "tenant-b",
            &vec![g.capabilities.clone(); 5],
            &mut ReplayCache::default(),
            100
        )
        .err(),
        Some(Denied::Scope)
    );
}
#[test]
fn tampered_signed_payload_is_rejected() {
    let g = grant();
    let mut e: SignedGrant = serde_json::from_slice(&signed(&g)).unwrap();
    let mut other = g.clone();
    other.tenant_id = "tenant-b".into();
    e.payload = STANDARD.encode(serde_json::to_vec(&other).unwrap());
    assert_eq!(
        verify(
            &g,
            &serde_json::to_vec(&e).unwrap(),
            "tenant-a",
            &vec![g.capabilities.clone(); 5],
            &mut ReplayCache::default(),
            100
        )
        .err(),
        Some(Denied::Signature)
    );
}
#[test]
fn replay_and_expiry_rejected() {
    let g = grant();
    let s = vec![g.capabilities.clone(); 5];
    let mut cache = ReplayCache::default();
    assert!(verify(&g, &signed(&g), "tenant-a", &s, &mut cache, 100).is_ok());
    assert_eq!(
        verify(&g, &signed(&g), "tenant-a", &s, &mut cache, 100).err(),
        Some(Denied::Replay)
    );
    assert_eq!(
        verify(&g, &signed(&g), "tenant-a", &s, &mut cache, 160).err(),
        Some(Denied::Expired)
    );
}
#[test]
fn view_only_denies_all_other_channels_and_expires_without_server() {
    let g = grant();
    let mut s = vec![g.capabilities.clone(); 5];
    s[3] = caps(&[Capability::ViewScreen]);
    let mut gate = verify(
        &g,
        &signed(&g),
        "tenant-a",
        &s,
        &mut ReplayCache::default(),
        100,
    )
    .unwrap();
    assert!(gate.check(Capability::ViewScreen, 1000).is_ok());
    for c in [
        Capability::ControlInput,
        Capability::FileReceive,
        Capability::FileSend,
        Capability::ClipboardRead,
        Capability::ClipboardWrite,
    ] {
        assert_eq!(gate.check(c, 1000), Err(Denied::Capability));
    }
    assert_eq!(
        gate.check(Capability::ViewScreen, 61_000),
        Err(Denied::Expired)
    );
    gate.revoke();
    assert_eq!(
        gate.check(Capability::ViewScreen, 1001),
        Err(Denied::Expired)
    );
}
#[test]
fn missing_os_permission_denies_capture() {
    let g = grant();
    let mut s = vec![g.capabilities.clone(); 5];
    s[4] = Capabilities::new();
    let gate = verify(
        &g,
        &signed(&g),
        "tenant-a",
        &s,
        &mut ReplayCache::default(),
        100,
    )
    .unwrap();
    assert_eq!(
        gate.check(Capability::ViewScreen, 1000),
        Err(Denied::Capability)
    );
}
#[test]
fn lease_must_not_exceed_sixty_seconds() {
    let mut g = grant();
    g.expires_at = 161;
    assert_eq!(
        verify(
            &g,
            &signed(&g),
            "tenant-a",
            &vec![g.capabilities.clone(); 5],
            &mut ReplayCache::default(),
            100
        )
        .err(),
        Some(Denied::Expired)
    );
}
#[test]
fn stale_input_and_disconnect_fail_closed() {
    let g = grant();
    let mut gate = verify(
        &g,
        &signed(&g),
        "tenant-a",
        &vec![g.capabilities.clone(); 5],
        &mut ReplayCache::default(),
        100,
    )
    .unwrap();
    let mut p = InputPacket {
        v: 1,
        session_id: g.session_id,
        epoch: 0,
        sequence: 1,
        event: InputEvent::Key {
            usage: 4,
            down: true,
        },
    };
    assert!(gate.validate_input(&p, 1000).is_ok());
    assert_eq!(gate.validate_input(&p, 1001), Err(Denied::StaleInput));
    p.sequence = 2;
    p.epoch = 1;
    assert_eq!(gate.validate_input(&p, 1002), Err(Denied::StaleInput));
    gate.disconnect();
    p.epoch = 0;
    assert_eq!(gate.validate_input(&p, 1003), Err(Denied::Expired));
}
#[test]
fn final_states_cannot_reopen() {
    for s in [SessionState::Ended, SessionState::Failed] {
        assert_eq!(s.transition(SessionState::Active), Err(Denied::State));
    }
    assert_eq!(
        SessionState::Requested.transition(SessionState::Active),
        Err(Denied::State)
    );
}
#[test]
fn dangerous_filenames_rejected() {
    for name in [
        "../x",
        "..\\x",
        "C:evil",
        "x:y",
        "CON.txt",
        "COM1",
        "LPT¹.log",
        "NUL",
        "x.",
        "a/b",
        "x\0y",
        "",
    ] {
        assert!(validate_filename(name).is_err(), "{name:?}");
    }
    assert!(validate_filename("müşteri raporu.pdf").is_ok());
}
proptest! {
    #[test] fn arbitrary_protocol_bytes_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..9000)) { let _=InputPacket::parse(&bytes); }
    #[test] fn invalid_coordinates_denied(x in any::<f64>(), y in any::<f64>()) { let p=InputPacket {v:1,session_id:"s".into(),epoch:0,sequence:1,event:InputEvent::Pointer{x,y,monitor:0}}; prop_assert_eq!(p.validate().is_ok(), x.is_finite() && y.is_finite() && (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y)); }
    #[test] fn arbitrary_filenames_never_panic(name in ".{0,400}") {let _=validate_filename(&name);}
}

#[test]
fn previous_connection_input_cannot_enter_a_new_grant() {
    let mut g = grant();
    g.input_epoch = 9;
    let mut gate = verify(
        &g,
        &signed(&g),
        "tenant-a",
        &vec![g.capabilities.clone(); 5],
        &mut ReplayCache::default(),
        100,
    )
    .unwrap();
    let mut packet = InputPacket {
        v: 1,
        session_id: g.session_id,
        epoch: 8,
        sequence: 100,
        event: InputEvent::Key {
            usage: 4,
            down: true,
        },
    };
    assert_eq!(gate.validate_input(&packet, 1000), Err(Denied::StaleInput));
    packet.epoch = 9;
    assert!(gate.validate_input(&packet, 1001).is_ok());
}

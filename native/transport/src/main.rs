//! Same-host native WebRTC experiments. No untrusted signaling endpoint.
use anyhow::{bail, Context, Result};
use bytes::{Bytes, BytesMut};
use clap::{Parser, Subcommand};
use rtc::{
    media::{io::sample_builder::SampleBuilder, Sample},
    media_stream::MediaStreamTrack,
    rtp::codec::h264::H264Packet,
    rtp_transceiver::rtp_sender::{
        RTCRtpCodec, RTCRtpCodecParameters, RTCRtpCodingParameters, RTCRtpEncodingParameters,
        RtpCodecKind,
    },
};
use std::{
    path::PathBuf,
    process::Stdio,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{mpsc, oneshot},
    time::timeout,
};
use webrtc::{
    data_channel::{DataChannel, DataChannelEvent},
    media_stream::{
        track_local::{static_sample::TrackLocalStaticSample, TrackLocal},
        track_remote::{TrackRemote, TrackRemoteEvent},
    },
    peer_connection::{
        register_default_interceptors, MediaEngine, PeerConnection, PeerConnectionBuilder,
        PeerConnectionEventHandler, RTCConfiguration, RTCConfigurationBuilder,
        RTCIceGatheringState, RTCIceServer, RTCIceTransportPolicy, Registry, SettingEngine,
    },
};

#[derive(Parser)]
#[command(
    version,
    about = "dengeX Phase 0 — same-host experiments, not a remote support release"
)]
struct Args {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    PilotReceive {
        #[arg(long)]
        helper: PathBuf,
        #[arg(long)]
        report: PathBuf,
    },
    Transport {
        #[arg(long)]
        relay: bool,
        #[arg(long)]
        report: Option<PathBuf>,
    },
    Native {
        #[arg(long)]
        helper: PathBuf,
        #[arg(long,default_value_t=15,value_parser=clap::value_parser!(u64).range(2..=60))]
        seconds: u64,
        #[arg(long)]
        synthetic: bool,
        #[arg(long)]
        start_local_test: bool,
        #[arg(long)]
        relay: bool,
        #[arg(long)]
        report: Option<PathBuf>,
    },
}
fn config(relay: bool) -> Result<RTCConfiguration> {
    let mut b = RTCConfigurationBuilder::default();
    if relay {
        let url = std::env::var("DX_TURN_URL").context("DX_TURN_URL required for --relay")?;
        if !(url.starts_with("turn:") || url.starts_with("turns:")) {
            bail!("invalid TURN URL");
        }
        if url.starts_with("turns:") || url.contains("transport=tcp") {
            bail!("TURN_TCP_TLS_UNSUPPORTED: webrtc-rs 0.20.5 currently gathers UDP TURN only; direct fallback is forbidden in relay mode");
        }
        b = b
            .with_ice_servers(vec![RTCIceServer {
                urls: vec![url],
                username: std::env::var("DX_TURN_USERNAME").context("TURN username missing")?,
                credential: std::env::var("DX_TURN_CREDENTIAL")
                    .context("TURN credential missing")?,
            }])
            .with_ice_transport_policy(RTCIceTransportPolicy::Relay);
    }
    Ok(b.build())
}
fn codec() -> RTCRtpCodec {
    RTCRtpCodec {
        mime_type: "video/H264".into(),
        clock_rate: 90000,
        channels: 0,
        sdp_fmtp_line: "level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e01f"
            .into(),
        rtcp_feedback: vec![],
    }
}
struct Handler {
    gather: mpsc::Sender<()>,
    video: mpsc::Sender<Bytes>,
    received: Arc<AtomicU64>,
    dropped: Arc<AtomicU64>,
}
#[async_trait::async_trait]
impl PeerConnectionEventHandler for Handler {
    async fn on_ice_gathering_state_change(&self, state: RTCIceGatheringState) {
        if state == RTCIceGatheringState::Complete {
            let _ = self.gather.try_send(());
        }
    }
    async fn on_data_channel(&self, dc: Arc<dyn DataChannel>) {
        tokio::spawn(async move {
            while let Some(e) = dc.poll().await {
                match e {
                    DataChannelEvent::OnMessage(m) if m.data.as_ref() == b"dx-probe-v1" => {
                        let _ = dc.send(m.data).await;
                    }
                    DataChannelEvent::OnClose => break,
                    _ => {}
                }
            }
        });
    }
    async fn on_track(&self, remote: Arc<dyn TrackRemote>) {
        let tx = self.video.clone();
        let r = self.received.clone();
        let d = self.dropped.clone();
        tokio::spawn(async move {
            let mut builder = SampleBuilder::new(1024, H264Packet::default(), 90000)
                .with_max_time_delay(Duration::from_millis(100));
            while let Some(e) = remote.poll().await {
                if let TrackRemoteEvent::OnRtpPacket(packet) = e {
                    builder.push(packet);
                    while let Some(sample) = builder.pop() {
                        if sample.data.len() > 4_194_304 {
                            continue;
                        }
                        r.fetch_add(1, Ordering::Relaxed);
                        if tx.try_send(sample.data).is_err() {
                            d.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }
        });
    }
}
async fn peer(config: RTCConfiguration, handler: Handler) -> Result<Arc<dyn PeerConnection>> {
    let mut m = MediaEngine::default();
    m.register_codec(
        RTCRtpCodecParameters {
            rtp_codec: codec(),
            payload_type: 102,
        },
        RtpCodecKind::Video,
    )?;
    let registry = register_default_interceptors(Registry::new(), &mut m)?;
    let mut settings = SettingEngine::default();
    settings.set_include_loopback_candidate(true);
    Ok(Arc::new(
        PeerConnectionBuilder::new()
            .with_configuration(config)
            .with_handler(Arc::new(handler))
            .with_media_engine(m)
            .with_setting_engine(settings)
            .with_interceptor_registry(registry)
            .with_udp_addrs(vec!["0.0.0.0:0"])
            .build()
            .await?,
    ))
}
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    if let Command::PilotReceive { helper, report } = &args.command {
        return pilot_receive(helper, report).await;
    }
    let (relay, path) = match &args.command {
        Command::Transport { relay, report } | Command::Native { relay, report, .. } => {
            (*relay, report.clone())
        }
        Command::PilotReceive { .. } => unreachable!(),
    };
    let result = experiment(&args, relay).await;
    match result {
        Ok(mut report) => {
            report["schemaVersion"] = 1.into();
            report["engine"] = "webrtc-rs 0.20.5".into();
            report["relayRequired"] = relay.into();
            report["twoDeviceTest"] = false.into();
            let data = serde_json::to_vec_pretty(&report)?;
            if let Some(p) = path {
                std::fs::write(p, &data)?;
            }
            println!("{}", String::from_utf8(data)?);
            Ok(())
        }
        Err(e) => {
            if let Some(p) = path {
                std::fs::write(
                    p,
                    serde_json::to_vec_pretty(
                        &serde_json::json!({"schemaVersion":1,"engine":"webrtc-rs 0.20.5","result":"failed","reason":e.to_string(),"twoDeviceTest":false}),
                    )?,
                )?;
            }
            Err(e)
        }
    }
}
async fn experiment(args: &Args, relay: bool) -> Result<serde_json::Value> {
    let (atx, mut arx) = mpsc::channel(1);
    let (btx, mut brx) = mpsc::channel(1);
    let (video_tx, video_rx) = mpsc::channel(3);
    let received = Arc::new(AtomicU64::new(0));
    let dropped = Arc::new(AtomicU64::new(0));
    let a = peer(
        config(relay)?,
        Handler {
            gather: atx,
            video: video_tx.clone(),
            received: received.clone(),
            dropped: dropped.clone(),
        },
    )
    .await?;
    let b = peer(
        config(relay)?,
        Handler {
            gather: btx,
            video: video_tx,
            received: received.clone(),
            dropped: dropped.clone(),
        },
    )
    .await?;
    let result = run_peers(
        args,
        a.clone(),
        b.clone(),
        &mut arx,
        &mut brx,
        video_rx,
        received,
        dropped,
    )
    .await;
    let _ = a.close().await;
    let _ = b.close().await;
    result
}
#[allow(clippy::too_many_arguments)]
async fn run_peers(
    args: &Args,
    a: Arc<dyn PeerConnection>,
    b: Arc<dyn PeerConnection>,
    arx: &mut mpsc::Receiver<()>,
    brx: &mut mpsc::Receiver<()>,
    mut video_rx: mpsc::Receiver<Bytes>,
    received: Arc<AtomicU64>,
    dropped: Arc<AtomicU64>,
) -> Result<serde_json::Value> {
    let dc = a.create_data_channel("dx.probe.v1", None).await?;
    let (echo_tx, echo_rx) = oneshot::channel();
    tokio::spawn(async move {
        while let Some(e) = dc.poll().await {
            match e {
                DataChannelEvent::OnOpen => {
                    let _ = dc.send(BytesMut::from(&b"dx-probe-v1"[..])).await;
                }
                DataChannelEvent::OnMessage(m) if m.data.as_ref() == b"dx-probe-v1" => {
                    let _ = echo_tx.send(());
                    break;
                }
                DataChannelEvent::OnClose => break,
                _ => {}
            }
        }
    });
    let track = Arc::new(TrackLocalStaticSample::new(MediaStreamTrack::new(
        "dengex-lab".into(),
        "screen".into(),
        "Local screen test".into(),
        RtpCodecKind::Video,
        vec![RTCRtpEncodingParameters {
            rtp_coding_parameters: RTCRtpCodingParameters {
                ssrc: Some(1),
                ..Default::default()
            },
            codec: codec(),
            ..Default::default()
        }],
    ))?);
    if matches!(args.command, Command::Native { .. }) {
        a.add_track(track.clone() as Arc<dyn TrackLocal>).await?;
    }
    let started = Instant::now();
    a.set_local_description(a.create_offer(None).await?).await?;
    timeout(Duration::from_secs(15), arx.recv())
        .await
        .context("offer gathering timeout")?;
    b.set_remote_description(a.local_description().await.context("missing offer")?)
        .await?;
    b.set_local_description(b.create_answer(None).await?)
        .await?;
    timeout(Duration::from_secs(15), brx.recv())
        .await
        .context("answer gathering timeout")?;
    a.set_remote_description(b.local_description().await.context("missing answer")?)
        .await?;
    timeout(Duration::from_secs(20), echo_rx)
        .await
        .context("WebRTC data channel timeout")??;
    let handshake_ms = started.elapsed().as_millis();
    if let Command::Native {
        helper,
        seconds,
        start_local_test,
        synthetic,
        report,
        ..
    } = &args.command
    {
        let native_report = report.as_ref().map(|p| p.with_extension("native.json"));
        let mut command = tokio::process::Command::new(helper);
        command.args(["--bridge", "--seconds", &seconds.to_string()]);
        if *synthetic {
            command.arg("--codec-test");
        }
        if *start_local_test {
            command.arg("--start-local-test");
        }
        if let Some(p) = &native_report {
            command.arg("--report").arg(p);
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);
        let mut child = command.spawn().context("cannot launch native helper")?;
        let mut output = child.stdout.take().context("capture pipe")?;
        let mut input = child.stdin.take().context("decoder pipe")?;
        let writer = tokio::spawn(async move {
            while let Some(frame) = video_rx.recv().await {
                input.write_u32(frame.len() as u32).await?;
                input.write_all(&frame).await?;
            }
            Ok::<(), std::io::Error>(())
        });
        let video = async {
            loop {
                let length = match output.read_u32().await {
                    Ok(n) => n as usize,
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                    Err(e) => return Err(e.into()),
                };
                if length == 0 || length > 4_194_304 {
                    bail!("invalid native frame length");
                }
                let mut buffer = vec![0u8; length];
                output.read_exact(&mut buffer).await?;
                track
                    .write_sample(
                        1,
                        102,
                        &Sample {
                            data: buffer.into(),
                            duration: Duration::from_secs_f64(1.0 / 30.0),
                            ..Default::default()
                        },
                        &[],
                    )
                    .await?;
            }
            Ok::<(), anyhow::Error>(())
        };
        let outcome = timeout(Duration::from_secs(*seconds + 120), video).await;
        writer.abort();
        match outcome {
            Ok(x) => x?,
            Err(_) => {
                child.kill().await?;
                bail!("native consent/capture deadline exceeded");
            }
        }
        let status = child.wait().await?;
        let native_metrics = if let Some(path) = &native_report {
            serde_json::from_slice::<serde_json::Value>(
                &std::fs::read(path).context("native report missing")?,
            )?
        } else {
            serde_json::Value::Null
        };
        if !status.success() || (!native_metrics.is_null() && native_metrics["result"] != "passed")
        {
            bail!(
                "{}",
                native_metrics["reason"]
                    .as_str()
                    .unwrap_or("native_helper_failed")
            );
        }
        if received.load(Ordering::Relaxed) == 0 {
            bail!("no native video received");
        }
        return Ok(
            serde_json::json!({"experiment":"local_webrtc_native","result":"passed","syntheticSource":synthetic,
            "handshakeAndEchoMs":handshake_ms,"receivedAccessUnits":received.load(Ordering::Relaxed),
            "droppedAccessUnits":dropped.load(Ordering::Relaxed),"native":native_metrics}),
        );
    }
    Ok(
        serde_json::json!({"experiment":"local_webrtc_data_channel","result":"passed","handshakeAndEchoMs":handshake_ms,
        "screenCaptureTest":false,"codecTest":false,"inputTest":false}),
    )
}

// Development evidence receiver. Invitation is read from stdin, never argv/logs.
async fn pilot_receive(helper: &PathBuf, report: &std::path::Path) -> Result<()> {
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    let config: serde_json::Value = serde_json::from_str(&line)?;
    let url = config["serverUrl"].as_str().context("server URL")?;
    if !url.starts_with("https://") {
        bail!("HTTPS required")
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .build()?;
    let joined: serde_json::Value = client
        .post(format!("{url}/pilot/join"))
        .json(&serde_json::json!({"invite":config["invite"],"name":"Yerel doğrulama alıcısı"}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let token = joined["token"].as_str().context("token")?;
    let (tx, mut rx) = mpsc::channel(1);
    let (vtx, mut vrx) = mpsc::channel(3);
    let received = Arc::new(AtomicU64::new(0));
    let dropped = Arc::new(AtomicU64::new(0));
    let pc = peer(
        RTCConfigurationBuilder::default()
            .with_ice_servers(vec![RTCIceServer {
                urls: vec!["stun:stun.cloudflare.com:3478".into()],
                ..Default::default()
            }])
            .build(),
        Handler {
            gather: tx,
            video: vtx,
            received: received.clone(),
            dropped: dropped.clone(),
        },
    )
    .await?;
    let outcome=async {
        let mut room;
        let started=Instant::now();
        loop {
            room=client.get(format!("{url}/pilot/room")).bearer_auth(token).send().await?.error_for_status()?.json::<serde_json::Value>().await?;
            if !room["offer"].is_null(){break}
            if started.elapsed()>Duration::from_secs(90)||room["state"]=="ended"{bail!("no approved offer")}
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        pc.set_remote_description(serde_json::from_value(room["offer"].clone())?).await?;
        pc.set_local_description(pc.create_answer(None).await?).await?;
        timeout(Duration::from_secs(20),rx.recv()).await?;
        let answer=serde_json::to_value(pc.local_description().await.context("answer")?)?;
        client.post(format!("{url}/pilot/action")).bearer_auth(token).json(&serde_json::json!({"action":"answer","description":answer})).send().await?.error_for_status()?;
        let native=report.with_extension("native.json");
        let mut child=tokio::process::Command::new(helper).args(["--bridge","--receive-only","--seconds","20","--report"]).arg(&native).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).kill_on_drop(true).spawn()?;
        let mut input=child.stdin.take().context("receiver stdin")?;
        let mut tick=tokio::time::interval(Duration::from_secs(5));let deadline=tokio::time::sleep(Duration::from_secs(24));tokio::pin!(deadline);
        loop {tokio::select!{
            Some(frame)=vrx.recv()=>{if input.write_u32(frame.len() as u32).await.is_err(){break}
            if input.write_all(&frame).await.is_err(){break}},
            _=tick.tick()=>{let r:serde_json::Value=client.get(format!("{url}/pilot/room")).bearer_auth(token).send().await?.error_for_status()?.json().await?;if r["state"]!="active"{break}},
            _=&mut deadline=>break,
        }}
        let _=child.kill().await;let _=child.wait().await;
        let metrics:serde_json::Value=serde_json::from_slice(&std::fs::read(&native)?)?;
        if metrics["result"]!="passed"{bail!("native receiver did not decode")}
        let result=serde_json::json!({"result":"passed","signaling":"public_cloudflare_https","twoProcesses":true,"twoDeviceTest":false,"receivedAccessUnits":received.load(Ordering::Relaxed),"native":metrics});
        std::fs::write(report,serde_json::to_vec_pretty(&result)?)?;
        println!("{}",serde_json::to_string(&result)?);Ok::<(),anyhow::Error>(())
    }.await;
    let _ = client
        .post(format!("{url}/pilot/action"))
        .bearer_auth(token)
        .json(&serde_json::json!({"action":"end"}))
        .send()
        .await;
    let _ = pc.close().await;
    outcome
}

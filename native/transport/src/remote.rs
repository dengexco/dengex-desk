use anyhow::{bail, Context, Result};
use rtc::{
    media::Sample,
    media_stream::MediaStreamTrack,
    rtp_transceiver::rtp_sender::{
        RTCRtpCodec, RTCRtpCodecParameters, RTCRtpCodingParameters, RTCRtpEncodingParameters,
        RtpCodecKind,
    },
};
use std::{
    path::Path,
    process::Stdio,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{
    io::AsyncReadExt,
    sync::{mpsc, watch},
    time::{interval, timeout},
};
use webrtc::{
    media_stream::track_local::{static_sample::TrackLocalStaticSample, TrackLocal},
    peer_connection::{
        register_default_interceptors, MediaEngine, PeerConnection, PeerConnectionBuilder,
        PeerConnectionEventHandler, RTCConfigurationBuilder, RTCIceGatheringState, RTCIceServer,
        RTCSessionDescription, Registry, SettingEngine,
    },
};
struct Events(mpsc::Sender<()>);
#[async_trait::async_trait]
impl PeerConnectionEventHandler for Events {
    async fn on_ice_gathering_state_change(&self, state: RTCIceGatheringState) {
        if state == RTCIceGatheringState::Complete {
            let _ = self.0.try_send(());
        }
    }
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
#[derive(Clone)]
pub struct Host {
    pub pc: Arc<dyn PeerConnection>,
    track: Arc<TrackLocalStaticSample>,
}
impl Host {
    pub async fn prepare() -> Result<(Self, serde_json::Value)> {
        let mut media = MediaEngine::default();
        media.register_codec(
            RTCRtpCodecParameters {
                rtp_codec: codec(),
                payload_type: 102,
            },
            RtpCodecKind::Video,
        )?;
        let registry = register_default_interceptors(Registry::new(), &mut media)?;
        let (tx, mut rx) = mpsc::channel(1);
        let mut settings = SettingEngine::default();
        settings.set_include_loopback_candidate(true);
        let config = RTCConfigurationBuilder::default()
            .with_ice_servers(vec![RTCIceServer {
                urls: vec!["stun:stun.cloudflare.com:3478".into()],
                ..Default::default()
            }])
            .build();
        let pc: Arc<dyn PeerConnection> = Arc::new(
            PeerConnectionBuilder::new()
                .with_configuration(config)
                .with_handler(Arc::new(Events(tx)))
                .with_media_engine(media)
                .with_setting_engine(settings)
                .with_interceptor_registry(registry)
                .with_udp_addrs(vec!["0.0.0.0:0"])
                .build()
                .await?,
        );
        let track = Arc::new(TrackLocalStaticSample::new(MediaStreamTrack::new(
            "dengex-pilot".into(),
            "screen".into(),
            "Approved screen".into(),
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
        let result = async {
            pc.add_track(track.clone() as Arc<dyn TrackLocal>).await?;
            pc.set_local_description(pc.create_offer(None).await?)
                .await?;
            timeout(Duration::from_secs(20), rx.recv())
                .await
                .context("ICE toplama zaman aşımı")?;
            serde_json::to_value(pc.local_description().await.context("Teklif oluşmadı")?)
                .map_err(Into::into)
        }
        .await;
        match result {
            Ok(offer) => Ok((Self { pc, track }, offer)),
            Err(e) => {
                let _ = pc.close().await;
                Err(e)
            }
        }
    }
    pub async fn answer(&self, answer: serde_json::Value) -> Result<()> {
        let desc: RTCSessionDescription = serde_json::from_value(answer)?;
        self.pc.set_remote_description(desc).await?;
        Ok(())
    }
    pub async fn stream(
        &self,
        helper: &Path,
        mut stop: watch::Receiver<bool>,
        deadline: Arc<AtomicU64>,
        clock: Instant,
    ) -> Result<u64> {
        if *stop.borrow() || clock.elapsed().as_millis() as u64 >= deadline.load(Ordering::SeqCst) {
            bail!("Ekran paylaşım yetkisi sona erdi")
        }
        let mut child = tokio::process::Command::new(helper)
            .args([
                "--bridge",
                "--remote-session",
                "--start-local-test",
                "--seconds",
                "1800",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;
        // Keeping stdin open also acts as a parent-liveness pipe for the native helper.
        let _input = child.stdin.take().context("native control pipe")?;
        let mut output = child.stdout.take().context("native frame pipe")?;
        let mut ticker = interval(Duration::from_millis(100));
        let frames = AtomicU64::new(0);
        let stream = async {
            loop {
                let length = match output.read_u32().await {
                    Ok(n) => n as usize,
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                    Err(e) => return Err(e.into()),
                };
                if length == 0 || length > 4_194_304 {
                    bail!("Native kare boyutu geçersiz")
                }
                let mut frame = vec![0; length];
                output.read_exact(&mut frame).await?;
                if clock.elapsed().as_millis() as u64 >= deadline.load(Ordering::SeqCst) {
                    bail!("Oturum süresi doldu")
                }
                self.track
                    .write_sample(
                        1,
                        102,
                        &Sample {
                            data: frame.into(),
                            duration: Duration::from_secs_f64(1.0 / 30.0),
                            ..Default::default()
                        },
                        &[],
                    )
                    .await?;
                frames.fetch_add(1, Ordering::Relaxed);
            }
            Ok::<(), anyhow::Error>(())
        };
        tokio::pin!(stream);
        let result = loop {
            tokio::select! {
             r=&mut stream=>break r,
             _=stop.changed()=>break Ok(()),
             _=ticker.tick()=>if clock.elapsed().as_millis() as u64>=deadline.load(Ordering::SeqCst){break Err(anyhow::anyhow!("Sunucu yetki süresi doldu; paylaşım kapatıldı"));}
            }
        };
        let _ = child.kill().await;
        let _ = child.wait().await;
        let _ = self.pc.close().await;
        result?;
        Ok(frames.load(Ordering::Relaxed))
    }
}

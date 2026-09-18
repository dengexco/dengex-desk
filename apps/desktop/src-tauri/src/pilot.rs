use base64::{engine::general_purpose::STANDARD, Engine};
use dengex_agent_core::{Capability, ExpectedScope, ReplayCache, SessionGate};
use ed25519_dalek::VerifyingKey;
use serde_json::{json, Value};
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{Manager, State};
use tokio::sync::{watch, Mutex};

#[derive(Clone)]
struct Connection {
    api: reqwest::Client,
    url: String,
    token: String,
    authority: VerifyingKey,
    id: String,
    role: String,
    stop: watch::Sender<bool>,
    fingerprints: Arc<Mutex<(String, String)>>,
    replay: Arc<Mutex<ReplayCache>>,
}
#[derive(Default)]
pub struct PilotState {
    connection: Mutex<Option<Connection>>,
}
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn api() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(error)
}
fn validate_url(url: &str) -> Result<String, String> {
    let u = reqwest::Url::parse(url).map_err(|_| "Sunucu adresi geçersiz")?;
    if u.scheme() != "https"
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
        || u.query().is_some()
        || u.fragment().is_some()
        || u.path() != "/"
    {
        return Err("Yalnızca HTTPS sunucu adresi girin; yol, parola ve sorgu içermemeli.".into());
    }
    Ok(url.trim_end_matches('/').into())
}
async fn response(mut r: reqwest::Response) -> Result<Value, String> {
    let status = r.status();
    if r.content_length().unwrap_or(0) > 131072 {
        return Err("Sunucu yanıtı çok büyük".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = r.chunk().await.map_err(|_| "Sunucu yanıtı okunamadı")? {
        if bytes.len() + chunk.len() > 131072 {
            return Err("Sunucu yanıtı çok büyük".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let v: Value = serde_json::from_slice(&bytes).map_err(|_| "Sunucu beklenen yanıtı vermedi")?;
    if !status.is_success() {
        return Err(match v["error"].as_str().unwrap_or("") {
            "invitation_invalid_expired_or_used" => {
                "Davet kodu geçersiz, kullanılmış veya süresi dolmuş."
            }
            "session_expired" => "Oturum sona erdi. Yeni bir davet oluşturun.",
            "unauthorized" => "Sunucu bağlantı bilgisi geçersiz.",
            "invalid_transition" => "Oturum durumu değişti; yeniden kontrol edin.",
            _ => "Sunucu isteği reddetti. Bağlantıyı kontrol edip yeniden deneyin.",
        }
        .into());
    };
    Ok(v)
}
impl Connection {
    async fn get(&self) -> Result<Value, String> {
        response(
            self.api
                .get(format!("{}/pilot/room", self.url))
                .bearer_auth(&self.token)
                .send()
                .await
                .map_err(|_| "Sunucuya ulaşılamıyor")?,
        )
        .await
    }
    async fn action(&self, action: &str, desc: Option<Value>) -> Result<Value, String> {
        let mut v = json!({"action":action});
        if let Some(d) = desc {
            v["description"] = d
        }
        response(
            self.api
                .post(format!("{}/pilot/action", self.url))
                .bearer_auth(&self.token)
                .json(&v)
                .send()
                .await
                .map_err(|_| "Sunucuya ulaşılamıyor")?,
        )
        .await
    }
    async fn verify(&self, room: &Value) -> Result<u64, String> {
        if room["id"] != self.id || room["role"] != self.role {
            return Err("Oturum kimliği uyuşmuyor".into());
        }
        if room["state"] != "active" {
            return Ok(0);
        }
        let (local, peer) = self.fingerprints.lock().await.clone();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(error)?
            .as_secs();
        let allowed = [Capability::ViewScreen].into_iter().collect();
        let mut replay = self.replay.lock().await;
        SessionGate::verify(
            &serde_json::to_vec(&room["grant"]).map_err(error)?,
            &self.authority,
            ExpectedScope {
                tenant: "attended-pilot",
                device: &self.id,
                user: "invited-viewer",
                session: &self.id,
                policy: 1,
                input_epoch: 1,
                peer_fingerprint: &peer,
                local_fingerprint: &local,
            },
            &[allowed],
            &mut replay,
            now,
            0,
        )
        .map_err(error)?;
        let raw = STANDARD
            .decode(room["grant"]["payload"].as_str().ok_or("Yetki eksik")?)
            .map_err(error)?;
        let grant: dengex_agent_core::Grant = serde_json::from_slice(&raw).map_err(error)?;
        Ok(grant
            .expires_at
            .saturating_sub(now)
            .saturating_sub(1)
            .min(40))
    }
}
#[cfg(target_os = "macos")]
struct AbortOnDrop(tokio::task::AbortHandle);
#[cfg(target_os = "macos")]
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}
fn fingerprint(desc: &Value) -> Result<String, String> {
    let sdp = desc["sdp"].as_str().ok_or("SDP eksik")?;
    if sdp.len() > 65536 {
        return Err("SDP çok büyük".into());
    }
    let mut found = String::new();
    for line in sdp.lines() {
        if let Some(f) = line.trim().strip_prefix("a=fingerprint:sha-256 ") {
            let f = f.to_uppercase();
            if f.len() != 95
                || !f
                    .split(':')
                    .all(|s| s.len() == 2 && u8::from_str_radix(s, 16).is_ok())
            {
                return Err("DTLS kimliği geçersiz".into());
            }
            if !found.is_empty() && found != f {
                return Err("Çelişen DTLS kimliği".into());
            }
            found = f;
        }
    }
    if found.is_empty() {
        return Err("DTLS kimliği bulunamadı".into());
    }
    Ok(found)
}
fn connection(url: String, v: &Value, role: &str) -> Result<Connection, String> {
    let public = STANDARD
        .decode(v["authorityKey"].as_str().ok_or("Sunucu anahtarı eksik")?)
        .map_err(error)?;
    let authority =
        VerifyingKey::from_bytes(&public.try_into().map_err(|_| "Sunucu anahtarı geçersiz")?)
            .map_err(error)?;
    let (stop, _) = watch::channel(false);
    Ok(Connection {
        api: api()?,
        url,
        token: v["token"].as_str().ok_or("Oturum bilgisi eksik")?.into(),
        authority,
        id: v["room"]["id"]
            .as_str()
            .ok_or("Oturum kimliği eksik")?
            .into(),
        role: role.into(),
        stop,
        fingerprints: Arc::new(Mutex::new((String::new(), String::new()))),
        replay: Arc::new(Mutex::new(ReplayCache::default())),
    })
}
#[tauri::command]
pub async fn pilot_config(app: tauri::AppHandle) -> Result<Value, String> {
    let path = app
        .path()
        .resource_dir()
        .map_err(error)?
        .join("resources/pilot-server.json");
    let mut config: Value = std::fs::read(path)
        .ok()
        .and_then(|d| serde_json::from_slice(&d).ok())
        .unwrap_or(json!({"serverUrl":""}));
    config["canHost"] = json!(
        cfg!(target_os = "macos")
            && app
                .path()
                .app_local_data_dir()
                .map_err(error)?
                .join("pilot-host.json")
                .is_file()
    );
    config["platform"] = json!(std::env::consts::OS);
    Ok(config)
}
#[tauri::command]
pub async fn pilot_create(
    app: tauri::AppHandle,
    state: State<'_, PilotState>,
) -> Result<Value, String> {
    if !cfg!(target_os = "macos") {
        return Err("Bu pilotta ekran kaynağı macOS uygulamasıdır.".into());
    }
    let mut slot = state.connection.lock().await;
    if slot.is_some() {
        return Err("Önce mevcut denemeyi sonlandırın".into());
    }
    let data = std::fs::read(
        app.path()
            .app_local_data_dir()
            .map_err(error)?
            .join("pilot-host.json"),
    )
    .map_err(|_| "Yerel pilot sunucu yapılandırması bulunamadı")?;
    let c: Value = serde_json::from_slice(&data).map_err(|_| "Pilot yapılandırması okunamadı")?;
    let url = validate_url(c["serverUrl"].as_str().ok_or("Sunucu adresi eksik")?)?;
    let v = response(
        api()?
            .post(format!("{url}/pilot/create"))
            .bearer_auth(
                c["createToken"]
                    .as_str()
                    .ok_or("Sunucu kurulum anahtarı eksik")?,
            )
            .send()
            .await
            .map_err(|_| "Pilot sunucusuna erişilemedi")?,
    )
    .await?;
    *slot = Some(connection(url.clone(), &v, "host")?);
    Ok(json!({"room":v["room"],"invite":v["invite"],"serverUrl":url}))
}
#[tauri::command]
pub async fn pilot_join(
    state: State<'_, PilotState>,
    server_url: String,
    invite: String,
    name: String,
) -> Result<Value, String> {
    let mut slot = state.connection.lock().await;
    if slot.is_some() {
        return Err("Önce mevcut denemeyi sonlandırın".into());
    }
    let url = validate_url(&server_url)?;
    let v = response(
        api()?
            .post(format!("{url}/pilot/join"))
            .json(&json!({"invite":invite.trim(),"name":name.trim()}))
            .send()
            .await
            .map_err(|_| "Pilot sunucusuna erişilemedi")?,
    )
    .await?;
    *slot = Some(connection(url, &v, "viewer")?);
    Ok(v["room"].clone())
}
async fn current(state: &PilotState) -> Result<Connection, String> {
    state
        .connection
        .lock()
        .await
        .clone()
        .ok_or("Açık deneme yok".into())
}
#[tauri::command]
pub async fn pilot_poll(state: State<'_, PilotState>) -> Result<Value, String> {
    let c = current(&state).await?;
    let mut room = c.get().await?;
    if c.role == "viewer" && room["state"] == "active" {
        room["leaseSeconds"] = c.verify(&room).await?.into()
    }
    room.as_object_mut()
        .ok_or("Geçersiz yanıt")?
        .remove("grant");
    Ok(room)
}
#[tauri::command]
pub async fn pilot_answer(
    state: State<'_, PilotState>,
    description: Value,
) -> Result<Value, String> {
    let c = current(&state).await?;
    if c.role != "viewer" || description["type"] != "answer" {
        return Err("Yanıt bu işlemde kullanılamaz".into());
    }
    let room = c.get().await?;
    *c.fingerprints.lock().await = (fingerprint(&description)?, fingerprint(&room["offer"])?);
    let mut room = c.action("answer", Some(description)).await?;
    room["leaseSeconds"] = c.verify(&room).await?.into();
    room.as_object_mut().unwrap().remove("grant");
    Ok(room)
}
#[tauri::command]
pub async fn pilot_stop(state: State<'_, PilotState>) -> Result<(), String> {
    if let Some(c) = state.connection.lock().await.take() {
        c.stop.send_replace(true);
        let _ = c.action("end", None).await;
    }
    Ok(())
}
pub fn close(state: &PilotState) {
    if let Ok(c) = state.connection.try_lock() {
        if let Some(c) = c.as_ref() {
            c.stop.send_replace(true);
        }
    }
}
#[tauri::command]
pub async fn pilot_accept(
    app: tauri::AppHandle,
    state: State<'_, PilotState>,
) -> Result<(), String> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, state);
        Err("Ekran paylaşımı bu pilotta yalnızca Mac'te desteklenir".into())
    }
    #[cfg(target_os = "macos")]
    {
        let c = current(&state).await?;
        if c.role != "host" {
            return Err("Yerel onay yalnızca ekranı paylaşan cihazdan verilir".into());
        }
        let helper = super::resource(&app, "dx-macos-probe")?;
        if super::read_status(&helper)?["screenRecording"] != true {
            return Err("Önce İzinler bölümünden ekran kaydı iznini açın".into());
        }
        c.action("accept", None).await?;
        tauri::async_runtime::spawn(async move {
            use std::sync::atomic::{AtomicU64, Ordering};
            let result=async{
    let (host,offer)=dengex_transport::remote::Host::prepare().await.map_err(error)?;
    let mut stopped=c.stop.subscribe();
    let outcome=async{
     let local=fingerprint(&offer)?;c.action("offer",Some(offer)).await?;
     let started=std::time::Instant::now();
     let active=loop{
      if *stopped.borrow()||started.elapsed()>Duration::from_secs(90){return Err("Bağlantı kurulamadı".into())}
      let r=c.get().await?;if r["state"]=="ended"{return Err("Deneme sonlandırıldı".into())}
      if r["state"]=="active"{break r}
      tokio::select!{_=tokio::time::sleep(Duration::from_secs(1))=>{},_=stopped.changed()=>return Err("Deneme sonlandırıldı".into())}
     };
     *c.fingerprints.lock().await=(local,fingerprint(&active["answer"])?);
     let lease=c.verify(&active).await?;host.answer(active["answer"].clone()).await.map_err(error)?;
     let clock=std::time::Instant::now();let deadline=Arc::new(AtomicU64::new(lease*1000));
     let stream_host=host.clone();let stream_stop=stopped.clone();let stream_deadline=deadline.clone();
     let mut stream=tokio::spawn(async move {stream_host.stream(&helper,stream_stop,stream_deadline,clock).await});
     let _abort=AbortOnDrop(stream.abort_handle());
     let mut tick=tokio::time::interval(Duration::from_secs(5));
     loop{tokio::select!{
      r=&mut stream=>{r.map_err(error)?.map_err(error)?;return Ok::<(),String>(())},
      _=stopped.changed()=>return Ok(()),
      _=tick.tick()=>{
       if let Ok(r)=c.get().await{
        if r["state"]!="active"{return Ok(())}
        let remaining=c.verify(&r).await?;
        deadline.store(clock.elapsed().as_millis() as u64+remaining*1000,Ordering::SeqCst);
       }
      }
     }}
    }.await;
    c.stop.send_replace(true);let _=host.pc.close().await;outcome
   }.await;
            let _ = c.action("end", None).await;
            if let Err(message) = result {
                use tauri::Emitter;
                let _ = app.emit("pilot-error", message);
            }
        });
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn urls_cannot_carry_credentials_or_downgrade() {
        for v in [
            "http://localhost:8080",
            "https://user:pass@example.com",
            "https://example.com/path",
            "https://example.com/?token=a",
        ] {
            assert!(validate_url(v).is_err())
        }
        assert_eq!(
            validate_url("https://test.trycloudflare.com/").unwrap(),
            "https://test.trycloudflare.com"
        );
    }
}

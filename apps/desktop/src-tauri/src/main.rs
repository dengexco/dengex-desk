#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod pilot;
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::{Manager, State};
struct LabState {
    busy: AtomicBool,
}
fn resource(app: &tauri::AppHandle, name: &str) -> Result<PathBuf, String> {
    let path = app
        .path()
        .resource_dir()
        .map_err(|_| "Kaynak klasörü bulunamadı")?
        .join("resources")
        .join(name);
    if !path.is_file() {
        return Err(format!("Native bileşen paketlenmemiş: {name}"));
    }
    Ok(path)
}
fn read_status(helper: &Path) -> Result<serde_json::Value, String> {
    let output = Command::new(helper)
        .arg("--status")
        .output()
        .map_err(|_| "Native izin sorgusu çalışmadı")?;
    if !output.status.success() {
        return Err("Native izin sorgusu başarısız".into());
    }
    serde_json::from_slice(&output.stdout).map_err(|_| "Native yanıt okunamadı".into())
}
#[tauri::command]
async fn native_status(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    if !cfg!(target_os = "macos") {
        return Ok(
            serde_json::json!({"platform":std::env::consts::OS,"screenRecording":false,"accessibility":false,"unsupported":true}),
        );
    }
    tauri::async_runtime::spawn_blocking(move || read_status(&resource(&app, "dx-macos-probe")?))
        .await
        .map_err(|_| "İzin sorgusu beklenmedik biçimde kapandı".to_string())?
}
#[tauri::command]
async fn device_identity(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    if !cfg!(target_os = "macos") {
        return Ok(serde_json::json!({"viewerOnly":true}));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let output = Command::new(resource(&app, "dx-device-identity")?).output()
            .map_err(|_| "Cihaz kimliği bileşeni başlatılamadı")?;
        if !output.status.success() {
            return Err("Cihaz kimliği Anahtar Zinciri'nden okunamadı. Mac'in kilidini açıp Anahtar Zinciri erişim isteğini kontrol edin; mevcut kimlik değiştirilmedi.".into());
        }
        serde_json::from_slice(&output.stdout).map_err(|_| "Cihaz kimliği okunamadı".into())
    }).await.map_err(|_| "Cihaz kimliği işlemi kapandı".to_string())?
}
#[tauri::command]
async fn request_permission(app: tauri::AppHandle, permission: String) -> Result<(), String> {
    if !cfg!(target_os = "macos") || !["screen", "input"].contains(&permission.as_str()) {
        return Err("Bu izin işlemi desteklenmiyor".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let output = Command::new(resource(&app, "dx-macos-probe")?)
            .args(["--request-permission", &permission])
            .output()
            .map_err(|_| "Sistem izinleri açılamadı")?;
        if output.status.success() {
            Ok(())
        } else {
            Err("Sistem Ayarları → Gizlilik ve Güvenlik bölümünden uygulamaya izin verin.".into())
        }
    })
    .await
    .map_err(|_| "İzin isteği kapandı".to_string())?
}
fn permission_block(kind: &str, status: &serde_json::Value) -> Option<serde_json::Value> {
    let required = match kind {
        "capture" => Some(("screenRecording", "screen_recording_permission_required")),
        "input" => Some(("accessibility", "accessibility_permission_required")),
        _ => None,
    };
    required.filter(|(field, _)| status[*field] != true).map(|(_, reason)|
        serde_json::json!({"schemaVersion":1,"experiment":kind,"result":"blocked","reason":reason,"twoDeviceTest":false}))
}
fn read_probe_result(report: &Path, output: &Output) -> Result<serde_json::Value, String> {
    // Failure reports are useful evidence too. Do not replace a native reason with
    // a generic error just because the subprocess returned a non-zero exit code.
    if report.is_file() {
        let bytes = std::fs::read(report).map_err(|_| "Test raporu okunamadı")?;
        let mut result: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| "Test raporu geçersiz".to_string())?;
        if !output.status.success() && result["result"] == "passed" {
            result["result"] = "failed".into();
            result["reason"] = "probe_process_failed".into();
        }
        return Ok(result);
    }
    Err("Test süreci rapor üretmeden kapandı. Uygulamayı yeniden açıp tekrar deneyin.".into())
}
#[tauri::command]
async fn run_probe(
    app: tauri::AppHandle,
    state: State<'_, LabState>,
    kind: String,
) -> Result<serde_json::Value, String> {
    if !["transport", "synthetic", "capture", "input"].contains(&kind.as_str()) {
        return Err("Bilinmeyen test".into());
    }
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err("Bir test zaten çalışıyor".into());
    }
    let result =
        tauri::async_runtime::spawn_blocking(move || -> Result<serde_json::Value, String> {
            let dir = app
                .path()
                .app_local_data_dir()
                .map_err(|_| "Rapor klasörü bulunamadı")?
                .join("reports");
            std::fs::create_dir_all(&dir).map_err(|_| "Rapor klasörü oluşturulamadı")?;
            let report = dir.join(format!("{kind}.json"));
            for path in [&report, &report.with_extension("native.json")] {
                if path.exists() {
                    std::fs::remove_file(path).map_err(|_| "Eski rapor temizlenemedi")?;
                }
            }
            let helper = if kind != "transport" {
                Some(resource(&app, "dx-macos-probe")?)
            } else {
                None
            };
            if let Some(helper) = &helper {
                if let Some(blocked) = permission_block(&kind, &read_status(helper)?) {
                    std::fs::write(&report, serde_json::to_vec_pretty(&blocked).unwrap())
                        .map_err(|_| "İzin sonucu kaydedilemedi")?;
                    return Ok(blocked);
                }
            }
            let mut c = if kind == "input" {
                let mut c = Command::new(helper.as_ref().unwrap());
                c.arg("--input-test");
                c
            } else {
                let mut c = Command::new(resource(
                    &app,
                    if cfg!(windows) {
                        "dx-probe.exe"
                    } else {
                        "dx-probe"
                    },
                )?);
                if kind == "transport" {
                    c.arg("transport");
                } else {
                    c.arg("native")
                        .arg("--helper")
                        .arg(helper.unwrap())
                        .args(["--seconds", "15"]);
                    if kind == "synthetic" {
                        c.arg("--synthetic");
                    }
                    // The explicitly labelled UI button is the local consent. The
                    // native window remains visible with Stop throughout the test.
                    if kind == "capture" {
                        c.arg("--start-local-test");
                    }
                }
                c
            };
            let output = c
                .arg("--report")
                .arg(&report)
                .output()
                .map_err(|_| "Test süreci başlatılamadı")?;
            read_probe_result(&report, &output)
        })
        .await;
    state.busy.store(false, Ordering::SeqCst);
    result.map_err(|_| "Test süreci beklenmedik biçimde kapandı".to_string())?
}
fn main() {
    tauri::Builder::default()
        .manage(pilot::PilotState::default())
        .on_window_event(|window, event| {
            if matches!(
                event,
                tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed
            ) {
                pilot::close(&window.state::<pilot::PilotState>());
            }
        })
        .manage(LabState {
            busy: AtomicBool::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            native_status,
            device_identity,
            request_permission,
            run_probe,
            pilot::pilot_config,
            pilot::pilot_create,
            pilot::pilot_join,
            pilot::pilot_poll,
            pilot::pilot_answer,
            pilot::pilot_accept,
            pilot::pilot_stop
        ])
        .run(tauri::generate_context!())
        .expect("desktop runtime failed");
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_or_unknown_permissions_block_only_relevant_tests() {
        let unknown = serde_json::json!({});
        assert_eq!(
            permission_block("capture", &unknown).unwrap()["reason"],
            "screen_recording_permission_required"
        );
        assert_eq!(
            permission_block("input", &unknown).unwrap()["reason"],
            "accessibility_permission_required"
        );
        assert!(permission_block("synthetic", &unknown).is_none());
        assert!(permission_block("transport", &unknown).is_none());
        let screen_only = serde_json::json!({"screenRecording": true, "accessibility": false});
        assert!(permission_block("capture", &screen_only).is_none());
        assert!(permission_block("input", &screen_only).is_some());
    }
    #[test]
    fn failed_child_keeps_specific_report_and_cannot_claim_success() {
        let report =
            std::env::temp_dir().join(format!("dx-probe-result-{}.json", std::process::id()));
        let output = Command::new("false").output().unwrap();
        std::fs::write(
            &report,
            br#"{"result":"failed","reason":"screen_recording_permission_required"}"#,
        )
        .unwrap();
        assert_eq!(
            read_probe_result(&report, &output).unwrap()["reason"],
            "screen_recording_permission_required"
        );
        std::fs::write(&report, br#"{"result":"passed"}"#).unwrap();
        assert_eq!(
            read_probe_result(&report, &output).unwrap()["result"],
            "failed"
        );
        std::fs::remove_file(report).unwrap();
    }
}

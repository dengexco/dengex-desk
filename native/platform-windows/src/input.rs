//! Text-only SendInput spike. Not wired to any remote channel. The eventual
//! dispatcher must call SessionGate and handle UIPI/secure desktop explicitly.
use windows::{
    core::{Error, Result},
    Win32::{Foundation::E_FAIL, UI::Input::KeyboardAndMouse::*},
};
/// Caller must own an explicitly approved interactive desktop experiment.
/// This adapter is not exposed as a Tauri command or network endpoint.
pub fn send_unicode_text(text: &str) -> Result<()> {
    if text.is_empty() || text.len() > 512 || text.contains('\0') {
        return Err(Error::new(E_FAIL, "invalid text payload"));
    }
    let mut events = Vec::with_capacity(text.len() * 2);
    for unit in text.encode_utf16() {
        for up in [false, true] {
            events.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: unit,
                        dwFlags: if up {
                            KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
                        } else {
                            KEYEVENTF_UNICODE
                        },
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            });
        }
    }
    let sent = unsafe { SendInput(&events, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != events.len() {
        // SendInput may fail through UIPI without a useful GetLastError.
        return Err(Error::new(
            E_FAIL,
            "SendInput incomplete; local approval/integrity boundary may prevent input",
        ));
    }
    Ok(())
}

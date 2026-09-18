//! Windows inbox Media Foundation H.264 encoder. All COM calls stay on one worker.
use std::{marker::PhantomData, mem::ManuallyDrop, rc::Rc};
use windows::{
    core::{Error, Interface, Result},
    Win32::{
        Foundation::E_FAIL,
        Media::MediaFoundation::*,
        System::{Com::*, Variant::VARIANT},
    },
};

pub struct Runtime(PhantomData<Rc<()>>);
impl Runtime {
    pub fn start() -> Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
            if let Err(e) = MFStartup(MF_VERSION, MFSTARTUP_FULL) {
                CoUninitialize();
                return Err(e);
            }
            Ok(Self(PhantomData))
        }
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        unsafe {
            let _ = MFShutdown();
            CoUninitialize();
        }
    }
}
pub struct Encoder {
    transform: IMFTransform,
    width: u32,
    height: u32,
    frame: i64,
    header: Vec<u8>,
}
fn fail(message: &str) -> Error {
    Error::new(E_FAIL, message)
}
fn pair(a: u32, b: u32) -> u64 {
    ((a as u64) << 32) | b as u64
}
impl Encoder {
    pub fn new(width: u32, height: u32) -> Result<Self> {
        unsafe {
            if width < 2
                || height < 2
                || width > 1280
                || height > 720
                || !width.is_multiple_of(2)
                || !height.is_multiple_of(2)
            {
                return Err(fail("invalid encoder dimensions"));
            }
            let transform: IMFTransform =
                CoCreateInstance(&CMSH264EncoderMFT, None, CLSCTX_INPROC_SERVER)?;
            let codec: ICodecAPI = transform.cast()?;
            codec.SetValue(&CODECAPI_AVLowLatencyMode, &VARIANT::from(true))?;
            codec.SetValue(&CODECAPI_AVEncMPVGOPSize, &VARIANT::from(20u32))?;
            let output = MFCreateMediaType()?;
            output.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            output.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)?;
            output.SetUINT32(&MF_MT_AVG_BITRATE, 2_000_000)?;
            output.SetUINT64(&MF_MT_FRAME_SIZE, pair(width, height))?;
            output.SetUINT64(&MF_MT_FRAME_RATE, pair(20, 1))?;
            output.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pair(1, 1))?;
            output.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
            output.SetUINT32(&MF_MT_MPEG2_PROFILE, eAVEncH264VProfile_Base.0 as u32)?;
            transform.SetOutputType(0, &output, 0)?;
            let input = MFCreateMediaType()?;
            input.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            input.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12)?;
            input.SetUINT64(&MF_MT_FRAME_SIZE, pair(width, height))?;
            input.SetUINT64(&MF_MT_FRAME_RATE, pair(20, 1))?;
            input.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pair(1, 1))?;
            input.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
            input.SetUINT32(&MF_MT_DEFAULT_STRIDE, width)?;
            input.SetUINT32(&MF_MT_SAMPLE_SIZE, width * height * 3 / 2)?;
            transform.SetInputType(0, &input, 0)?;
            transform.ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0)?;
            transform.ProcessMessage(MFT_MESSAGE_NOTIFY_START_OF_STREAM, 0)?;
            let mut encoder = Self {
                transform,
                width,
                height,
                frame: 0,
                header: Vec::new(),
            };
            encoder.refresh_header()?;
            Ok(encoder)
        }
    }
    fn refresh_header(&mut self) -> Result<()> {
        unsafe {
            let media = self.transform.GetOutputCurrentType(0)?;
            if let Ok(size) = media.GetBlobSize(&MF_MT_MPEG_SEQUENCE_HEADER) {
                if size > 65536 {
                    return Err(fail("oversized H264 sequence header"));
                }
                self.header.resize(size as usize, 0);
                media.GetBlob(&MF_MT_MPEG_SEQUENCE_HEADER, &mut self.header, None)?;
            }
            Ok(())
        }
    }
    pub fn encode(&mut self, data: &[u8]) -> Result<Vec<Vec<u8>>> {
        unsafe {
            if data.len() != self.width as usize * self.height as usize * 3 / 2 {
                return Err(fail("invalid NV12 frame"));
            }
            let sample = MFCreateSample()?;
            let buffer = MFCreateMemoryBuffer(data.len() as u32)?;
            let mut pointer = std::ptr::null_mut();
            buffer.Lock(&mut pointer, None, None)?;
            if pointer.is_null() {
                let _ = buffer.Unlock();
                return Err(fail("null encoder input buffer"));
            }
            std::ptr::copy_nonoverlapping(data.as_ptr(), pointer, data.len());
            buffer.Unlock()?;
            buffer.SetCurrentLength(data.len() as u32)?;
            sample.AddBuffer(&buffer)?;
            sample.SetSampleTime(self.frame * 500_000)?;
            sample.SetSampleDuration(500_000)?;
            self.transform.ProcessInput(0, &sample, 0)?;
            self.frame += 1;
            let mut frames = Vec::new();
            for _ in 0..16 {
                let info = self.transform.GetOutputStreamInfo(0)?;
                if info.cbSize > 4_194_304 {
                    return Err(fail("oversized encoder output"));
                }
                let sample = if info.dwFlags & MFT_OUTPUT_STREAM_PROVIDES_SAMPLES.0 as u32 == 0 {
                    let s = MFCreateSample()?;
                    s.AddBuffer(&MFCreateMemoryBuffer(info.cbSize.max(1_048_576))?)?;
                    Some(s)
                } else {
                    None
                };
                let mut output = [MFT_OUTPUT_DATA_BUFFER {
                    dwStreamID: 0,
                    pSample: ManuallyDrop::new(sample),
                    dwStatus: 0,
                    pEvents: ManuallyDrop::new(None),
                }];
                let mut status = 0;
                let result = self.transform.ProcessOutput(0, &mut output, &mut status);
                let sample = ManuallyDrop::take(&mut output[0].pSample);
                drop(ManuallyDrop::take(&mut output[0].pEvents));
                match result {
                    Err(e) if e.code() == MF_E_TRANSFORM_NEED_MORE_INPUT => break,
                    Err(e) if e.code() == MF_E_TRANSFORM_STREAM_CHANGE => {
                        self.refresh_header()?;
                        continue;
                    }
                    Err(e) => return Err(e),
                    Ok(()) => {}
                }
                let sample = sample.ok_or_else(|| fail("encoder produced no sample"))?;
                let buffer = sample.ConvertToContiguousBuffer()?;
                let mut pointer = std::ptr::null_mut();
                let mut length = 0;
                buffer.Lock(&mut pointer, None, Some(&mut length))?;
                if pointer.is_null() || length == 0 || length > 4_194_304 {
                    let _ = buffer.Unlock();
                    return Err(fail("invalid H264 output"));
                }
                let bytes = std::slice::from_raw_parts(pointer, length as usize).to_vec();
                buffer.Unlock()?;
                if !bytes.starts_with(&[0, 0, 1]) && !bytes.starts_with(&[0, 0, 0, 1]) {
                    return Err(fail("encoder did not return Annex B H264"));
                }
                // Repeating parameter sets lets a viewer recover on the next periodic IDR.
                if self.header.is_empty() {
                    self.refresh_header()?;
                }
                let mut access_unit = self.header.clone();
                access_unit.extend(bytes);
                frames.push(access_unit);
            }
            Ok(frames)
        }
    }
}
impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe {
            let _ = self.transform.ProcessMessage(MFT_MESSAGE_COMMAND_FLUSH, 0);
            let _ = self
                .transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_END_STREAMING, 0);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_inbox_encoder_emits_h264_from_synthetic_nv12() {
        let _runtime = Runtime::start().unwrap();
        let mut encoder = Encoder::new(128, 128).unwrap();
        let mut data = vec![128; 128 * 128 * 3 / 2];
        data[..128 * 128].fill(16);
        let mut frames = Vec::new();
        for _ in 0..5 {
            frames.extend(encoder.encode(&data).unwrap());
        }
        assert!(!frames.is_empty());
        let mut nals = Vec::new();
        for frame in frames {
            for i in 0..frame.len().saturating_sub(4) {
                if frame[i..].starts_with(&[0, 0, 1]) {
                    nals.push(frame[i + 3] & 31);
                }
            }
        }
        assert!(nals.contains(&7), "missing SPS");
        assert!(nals.contains(&8), "missing PPS");
        assert!(nals.contains(&5), "missing IDR");
    }
}

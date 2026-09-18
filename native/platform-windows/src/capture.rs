//! DXGI CPU-readback spike. Rotation, cursor compositing, access-loss recovery
//! and Media Foundation encoding must pass native acceptance before shipping.
use windows::{
    core::{Error, Interface, Result},
    Win32::{
        Foundation::{E_FAIL, HMODULE},
        Graphics::{
            Direct3D::D3D_DRIVER_TYPE_HARDWARE,
            Direct3D11::*,
            Dxgi::{Common::*, *},
        },
    },
};

pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}
pub struct Capture {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    duplication: IDXGIOutputDuplication,
}
impl Capture {
    /// Run only after a native consent/grant check in the interactive user desktop.
    pub fn open(output_index: u32) -> Result<Self> {
        unsafe {
            let (mut device, mut context) = (None, None);
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )?;
            let device = device.ok_or_else(|| Error::new(E_FAIL, "no D3D device"))?;
            let dxgi: IDXGIDevice = device.cast()?;
            let output: IDXGIOutput1 = dxgi.GetAdapter()?.EnumOutputs(output_index)?.cast()?;
            let duplication = output.DuplicateOutput(&device)?;
            if duplication.GetDesc().Rotation != DXGI_MODE_ROTATION_IDENTITY {
                return Err(Error::new(
                    E_FAIL,
                    "rotated monitor not yet supported by spike",
                ));
            }
            Ok(Self {
                device,
                context: context.ok_or_else(|| Error::new(E_FAIL, "no D3D context"))?,
                duplication,
            })
        }
    }
    /// Timeout is not a captured black frame: it remains an explicit DXGI error.
    pub fn next_frame(&self) -> Result<Frame> {
        unsafe {
            let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
            let mut resource = None;
            self.duplication
                .AcquireNextFrame(100, &mut info, &mut resource)?;
            let result = (|| {
                let texture: ID3D11Texture2D = resource
                    .ok_or_else(|| Error::new(E_FAIL, "no acquired texture"))?
                    .cast()?;
                let mut desc = D3D11_TEXTURE2D_DESC::default();
                texture.GetDesc(&mut desc);
                if desc.Width > 16384 || desc.Height > 16384 || desc.Width == 0 || desc.Height == 0
                {
                    return Err(Error::new(E_FAIL, "invalid display dimensions"));
                }
                if desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM {
                    return Err(Error::new(E_FAIL, "unsupported capture format"));
                }
                desc.Usage = D3D11_USAGE_STAGING;
                desc.BindFlags = 0;
                desc.MiscFlags = 0;
                desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
                let mut staging = None;
                self.device
                    .CreateTexture2D(&desc, None, Some(&mut staging))?;
                let staging = staging.ok_or_else(|| Error::new(E_FAIL, "no staging texture"))?;
                self.context.CopyResource(&staging, &texture);
                let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                self.context
                    .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;
                let row = desc.Width as usize * 4;
                let valid = !mapped.pData.is_null() && mapped.RowPitch as usize >= row;
                let mut bgra = Vec::new();
                if valid {
                    bgra.resize(row * desc.Height as usize, 0);
                    for y in 0..desc.Height as usize {
                        let src = std::slice::from_raw_parts(
                            (mapped.pData as *const u8).add(y * mapped.RowPitch as usize),
                            row,
                        );
                        bgra[y * row..(y + 1) * row].copy_from_slice(src);
                    }
                }
                self.context.Unmap(&staging, 0);
                if !valid {
                    return Err(Error::new(E_FAIL, "invalid mapped buffer"));
                }
                Ok(Frame {
                    width: desc.Width,
                    height: desc.Height,
                    bgra,
                })
            })();
            let released = self.duplication.ReleaseFrame();
            match result {
                Ok(frame) => {
                    released?;
                    Ok(frame)
                }
                Err(e) => Err(e),
            }
        }
    }
}

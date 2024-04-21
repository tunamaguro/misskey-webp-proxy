use anyhow::{Context, Ok, Result};
use image::{Frames, RgbaImage};
use libwebp_sys::{
    VP8StatusCode, WebPAnimEncoderAdd, WebPAnimEncoderAssemble, WebPAnimEncoderDelete,
    WebPAnimEncoderNewInternal, WebPAnimEncoderOptions, WebPAnimEncoderOptionsInitInternal,
    WebPConfig, WebPData, WebPDataClear, WebPEncode, WebPGetMuxABIVersion, WebPMemoryWrite,
    WebPMemoryWriter, WebPMemoryWriterClear, WebPMemoryWriterInit, WebPPicture, WebPPictureFree,
    WebPPictureImportRGBA, WebPPreset,
};

struct ManagedWebpMemoryWriter {
    wrt: WebPMemoryWriter,
}

impl ManagedWebpMemoryWriter {
    fn get(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.wrt.mem, self.wrt.size) }
    }
}

impl Drop for ManagedWebpMemoryWriter {
    fn drop(&mut self) {
        unsafe { WebPMemoryWriterClear(&mut self.wrt) };
    }
}

struct ManagedWebpPicture {
    config: WebPConfig,
    picture: WebPPicture,
}

impl ManagedWebpPicture {
    fn from_rgba(rgba_img: &RgbaImage, quality_factor: f32) -> Result<Self> {
        let config = WebPConfig::new_with_preset(WebPPreset::WEBP_PRESET_PICTURE, quality_factor)
            .map_err(|_| anyhow::anyhow!("WebPConfig init failed"))?;

        let mut picture =
            WebPPicture::new().map_err(|_| anyhow::anyhow!("WebPPicture init failed"))?;
        picture.height = rgba_img.height() as i32;
        picture.width = rgba_img.width() as i32;

        unsafe {
            WebPPictureImportRGBA(&mut picture, rgba_img.as_raw().as_ptr(), picture.width * 4);
        };
        Ok(Self { config, picture })
    }

    fn encode(mut self) -> Result<ManagedWebpMemoryWriter> {
        let mut wrt = std::mem::MaybeUninit::<WebPMemoryWriter>::uninit();
        unsafe { WebPMemoryWriterInit(wrt.as_mut_ptr()) };
        self.picture.writer = Some(WebPMemoryWrite);
        self.picture.custom_ptr = wrt.as_mut_ptr() as *mut std::ffi::c_void;
        let status = unsafe { WebPEncode(&self.config, &mut self.picture) };
        let wrt = unsafe { wrt.assume_init() };

        let mem_writer = ManagedWebpMemoryWriter { wrt };

        if status == VP8StatusCode::VP8_STATUS_OK as i32 {
            Ok(mem_writer)
        } else {
            Err(anyhow::anyhow!(format!(
                "WebpEncode error code: {}",
                status
            )))
        }
    }
}

impl Drop for ManagedWebpPicture {
    fn drop(&mut self) {
        unsafe { WebPPictureFree(&mut self.picture) }
    }
}

/// アニメーションを含まない画像をWebpにエンコードする
pub(crate) fn encode_webp_image(rgba_img: RgbaImage) -> Result<Vec<u8>> {
    let wrt = ManagedWebpPicture::from_rgba(&rgba_img, 75.0)?.encode()?;
    let buf = wrt.get();
    Ok(buf.into())
}

/// アニメーションをWebpにエンコードする
pub(crate) fn encode_webp_anim(frames: Frames) -> Result<Vec<u8>> {
    let frames = frames.into_iter().collect_frames()?;
    let first_frame = frames.first().context("cannot get first frame")?;
    let mut anim_option = std::mem::MaybeUninit::<WebPAnimEncoderOptions>::uninit();
    let mux_abi_version = WebPGetMuxABIVersion();
    unsafe { WebPAnimEncoderOptionsInitInternal(anim_option.as_mut_ptr(), mux_abi_version) };
    let encoder = unsafe {
        WebPAnimEncoderNewInternal(
            first_frame.buffer().width() as i32,
            first_frame.buffer().height() as i32,
            anim_option.as_ptr(),
            mux_abi_version,
        )
    };

    let mut time_stamp_ms = 0;
    for f in frames {
        let duration = f.delay().numer_denom_ms();
        time_stamp_ms += duration.0 / duration.1;
        let mut pic = ManagedWebpPicture::from_rgba(f.buffer(), 75_f32)?;
        let status = unsafe {
            WebPAnimEncoderAdd(encoder, &mut pic.picture, time_stamp_ms as i32, &pic.config)
        };
        // 0だと失敗
        if status == 0 {
            unsafe {
                WebPAnimEncoderDelete(encoder);
            };
            return Err(anyhow::anyhow!(format!(
                "Webp Anim encode faild: {}",
                status
            )));
        }
    }
    unsafe {
        WebPAnimEncoderAdd(
            encoder,
            core::ptr::null_mut(),
            time_stamp_ms as i32,
            std::ptr::null(),
        );
    };

    let mut webp_data = std::mem::MaybeUninit::<WebPData>::uninit();
    let status = unsafe { WebPAnimEncoderAssemble(encoder, webp_data.as_mut_ptr()) };
    // 0だと失敗
    if status == 0 {
        unsafe {
            WebPAnimEncoderDelete(encoder);
        };
        return Err(anyhow::anyhow!(format!(
            "Webp Anim encode faild: {}",
            status
        )));
    }
    unsafe {
        WebPAnimEncoderDelete(encoder);
    };

    let mut webp_data = unsafe { webp_data.assume_init() };
    let buf = unsafe { std::slice::from_raw_parts(webp_data.bytes, webp_data.size) };

    unsafe {
        WebPDataClear(&mut webp_data);
    };
    Ok(buf.into())
}

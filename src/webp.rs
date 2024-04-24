use anyhow::{Context, Ok, Result};
use image::{Frame, RgbaImage};
use libwebp_sys::{
    WebPAnimEncoderAdd, WebPAnimEncoderAssemble, WebPAnimEncoderDelete, WebPAnimEncoderNewInternal,
    WebPAnimEncoderOptions, WebPAnimEncoderOptionsInitInternal, WebPConfig, WebPData,
    WebPDataClear, WebPEncode, WebPGetMuxABIVersion, WebPMemoryWrite, WebPMemoryWriter,
    WebPMemoryWriterClear, WebPMemoryWriterInit, WebPMuxAnimParams, WebPMuxAssemble,
    WebPMuxCreateInternal, WebPMuxDelete, WebPMuxError, WebPMuxSetAnimationParams, WebPPicture,
    WebPPictureFree, WebPPictureImportRGBA, WebPPreset, WebPValidateConfig,
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
        let mut config =
            WebPConfig::new_with_preset(WebPPreset::WEBP_PRESET_PICTURE, quality_factor)
                .map_err(|_| anyhow::anyhow!("WebPConfig init failed"))?;
        config.alpha_compression = 0;
        if unsafe { WebPValidateConfig(&config) } == 0 {
            return Err(anyhow::anyhow!("WebpConfig Validate error"));
        }

        let mut picture =
            WebPPicture::new().map_err(|_| anyhow::anyhow!("WebPPicture init failed"))?;
        picture.use_argb = 1;
        picture.height = rgba_img.height() as i32;
        picture.width = rgba_img.width() as i32;

        let status = unsafe {
            WebPPictureImportRGBA(&mut picture, rgba_img.as_raw().as_ptr(), picture.width * 4)
        };
        if status == 0 {
            return Err(anyhow::anyhow!("Webp importRGBA failed"));
        }
        Ok(Self { config, picture })
    }

    fn lossless(mut self) -> Self {
        self.config.lossless = 1;
        self.config.alpha_compression = 0;
        return self;
    }

    fn near_lossless(mut self, near_lossless: i32) -> Self {
        self.config.near_lossless = near_lossless;
        return self;
    }

    fn encode(mut self) -> Result<ManagedWebpMemoryWriter> {
        let mut wrt = std::mem::MaybeUninit::<WebPMemoryWriter>::uninit();
        unsafe { WebPMemoryWriterInit(wrt.as_mut_ptr()) };
        self.picture.writer = Some(WebPMemoryWrite);
        self.picture.custom_ptr = wrt.as_mut_ptr() as _;
        let status = unsafe { WebPEncode(&self.config, &mut self.picture) };
        let wrt = unsafe { wrt.assume_init() };

        let mem_writer = ManagedWebpMemoryWriter { wrt };

        // 0の時エラー
        if status != 0 {
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
pub(crate) fn encode_webp_anim(frames: Vec<Frame>) -> Result<Vec<u8>> {
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
    let mux = unsafe { WebPMuxCreateInternal(webp_data.as_ptr(), 1, mux_abi_version) };
    let mux_error = unsafe {
        WebPMuxSetAnimationParams(
            mux,
            &WebPMuxAnimParams {
                bgcolor: 0,
                loop_count: 0,
            },
        )
    };
    let mut mux_data = unsafe { webp_data.assume_init() };
    unsafe { WebPDataClear(&mut mux_data) };
    let mut webp_data = std::mem::MaybeUninit::<WebPData>::uninit();
    let mux_error = unsafe { WebPMuxAssemble(mux, webp_data.as_mut_ptr()) };
    if mux_error != WebPMuxError::WEBP_MUX_OK {
        return Err(anyhow::anyhow!("mux error"));
    }
    let mut webp_data = unsafe { webp_data.assume_init() };
    let buf = unsafe { std::slice::from_raw_parts(webp_data.bytes, webp_data.size) };

    unsafe {
        WebPMuxDelete(mux);
        WebPAnimEncoderDelete(encoder);
        WebPDataClear(&mut webp_data);
    };
    Ok(buf.into())
}

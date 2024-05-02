use anyhow::Result;
use image::{Frame, RgbaImage};
use libwebp_sys::{
    WebPAnimEncoder, WebPAnimEncoderAdd, WebPAnimEncoderAssemble, WebPAnimEncoderDelete,
    WebPAnimEncoderNewInternal, WebPAnimEncoderOptions, WebPAnimEncoderOptionsInitInternal,
    WebPConfig, WebPData, WebPDataClear, WebPEncode, WebPGetMuxABIVersion, WebPMemoryWrite,
    WebPMemoryWriter, WebPMemoryWriterClear, WebPMemoryWriterInit, WebPMux, WebPMuxAnimParams,
    WebPMuxAssemble, WebPMuxCreateInternal, WebPMuxDelete, WebPMuxError, WebPMuxSetAnimationParams,
    WebPPicture, WebPPictureFree, WebPPictureImportRGBA, WebPPreset, WebPValidateConfig,
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
        self
    }

    fn near_lossless(mut self, near_lossless: i32) -> Self {
        self.config.near_lossless = near_lossless;
        self
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
pub(crate) fn encode_webp_image(rgba_img: RgbaImage, quality_factor: f32) -> Result<Vec<u8>> {
    let wrt = ManagedWebpPicture::from_rgba(&rgba_img, quality_factor)?.encode()?;
    let buf = wrt.get();
    Ok(buf.into())
}

struct ManagedWebpData {
    webp_data: WebPData,
}

impl ManagedWebpData {
    fn new(ptr: std::mem::MaybeUninit<WebPData>) -> Self {
        let webp_data = unsafe { ptr.assume_init() };
        Self {
            webp_data: webp_data,
        }
    }
}

impl Drop for ManagedWebpData {
    fn drop(&mut self) {
        unsafe {
            WebPDataClear(&mut self.webp_data);
        }
    }
}

struct ManagedWebpMux {
    mux: *mut WebPMux,
}

impl ManagedWebpMux {
    fn new(webp_data: &WebPData, mux_abi_ver: i32) -> Self {
        let mux = unsafe { WebPMuxCreateInternal(webp_data, 1, mux_abi_ver) };
        Self { mux }
    }
}

impl Drop for ManagedWebpMux {
    fn drop(&mut self) {
        unsafe {
            WebPMuxDelete(self.mux);
        }
    }
}

struct ManagedWebpAnim<'a> {
    anim_option: WebPAnimEncoderOptions,
    anim_encoder: *mut WebPAnimEncoder,
    webp_muxabi_ver: i32,
    frames: Box<dyn Iterator<Item = Frame> + 'a>,
}

impl<'a> ManagedWebpAnim<'a> {
    fn new(movie: (Frame, Box<dyn Iterator<Item = Frame> + 'a>)) -> Result<Self> {
        let (first_frame, other_frames) = movie;
        let mux_abi_version = WebPGetMuxABIVersion();
        let mut anim_option = std::mem::MaybeUninit::<WebPAnimEncoderOptions>::uninit();
        unsafe { WebPAnimEncoderOptionsInitInternal(anim_option.as_mut_ptr(), mux_abi_version) };
        let anim_option = unsafe { anim_option.assume_init() };
        let encoder = unsafe {
            WebPAnimEncoderNewInternal(
                first_frame.buffer().width() as i32,
                first_frame.buffer().height() as i32,
                &anim_option,
                mux_abi_version,
            )
        };

        return Ok(Self {
            anim_option,
            webp_muxabi_ver: mux_abi_version,
            anim_encoder: encoder,
            frames: other_frames,
        });
    }

    fn encode(self, quality_factor: f32) -> Result<Vec<u8>> {
        let mut time_stamp_ms = 0;
        for f in self.frames {
            Self::add_frame(quality_factor, f, &mut time_stamp_ms, self.anim_encoder);
        }

        let mut webp_data = std::mem::MaybeUninit::<WebPData>::uninit();
        let status = unsafe { WebPAnimEncoderAssemble(self.anim_encoder, webp_data.as_mut_ptr()) };
        if status == 0 {
            return Err(anyhow::anyhow!("Webp Anim Assemble failed: {}", status));
        }
        let mut webp_data = ManagedWebpData::new(webp_data);

        // mux
        let mux = ManagedWebpMux::new(&webp_data.webp_data, self.webp_muxabi_ver);
        Self::check_mux_error(unsafe {
            WebPMuxSetAnimationParams(
                mux.mux,
                &WebPMuxAnimParams {
                    bgcolor: 0,
                    loop_count: 0,
                },
            )
        })?;

        Self::check_mux_error(unsafe { WebPMuxAssemble(mux.mux, &mut webp_data.webp_data) })?;
        let buf = unsafe {
            std::slice::from_raw_parts(webp_data.webp_data.bytes, webp_data.webp_data.size)
        };
        let buf = buf.to_vec();
        Ok(buf)
    }

    fn check_mux_error(e: WebPMuxError) -> Result<()> {
        match e {
            WebPMuxError::WEBP_MUX_OK => Ok(()),
            _ => Err(anyhow::anyhow!("mux err")),
        }
    }

    fn add_frame(
        quality_factor: f32,
        f: Frame,
        time_stamp_ms: &mut u32,
        anim_encoder: *mut WebPAnimEncoder,
    ) -> Result<()> {
        let duration = f.delay().numer_denom_ms();
        *time_stamp_ms += duration.0 / duration.1;
        let mut pic = ManagedWebpPicture::from_rgba(f.buffer(), quality_factor)?;
        let status = unsafe {
            WebPAnimEncoderAdd(
                anim_encoder,
                &mut pic.picture,
                *time_stamp_ms as i32,
                &pic.config,
            )
        };
        if status == 0 {
            return Err(anyhow::anyhow!(format!(
                "Webp Anim encode faild: {}",
                status
            )));
        }
        Ok(())
    }
}

impl<'a> Drop for ManagedWebpAnim<'a> {
    fn drop(&mut self) {
        unsafe {
            WebPAnimEncoderDelete(self.anim_encoder);
        }
    }
}

/// アニメーションをWebpにエンコードする
pub(crate) fn encode_webp_anim<'a>(
    movie: (Frame, Box<dyn Iterator<Item = Frame> + 'a>),
    quality_factor: f32,
) -> Result<Vec<u8>> {
    let encoder = ManagedWebpAnim::new(movie)?;
    encoder.encode(quality_factor)
}

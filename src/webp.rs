use std::marker::PhantomData;

use anyhow::{Context, Ok, Result};
use image::{Frame, RgbaImage};
use libwebp_sys::{
    WebPAnimEncoder, WebPAnimEncoderAdd, WebPAnimEncoderAssemble, WebPAnimEncoderDelete,
    WebPAnimEncoderNewInternal, WebPAnimEncoderOptions, WebPAnimEncoderOptionsInitInternal,
    WebPConfig, WebPData, WebPDataClear, WebPEncode, WebPGetMuxABIVersion, WebPMemoryWrite,
    WebPMemoryWriter, WebPMemoryWriterClear, WebPMemoryWriterInit, WebPMux, WebPMuxAnimParams,
    WebPMuxAssemble, WebPMuxCreateInternal, WebPMuxDelete, WebPMuxError, WebPMuxSetAnimationParams,
    WebPPicture, WebPPictureFree, WebPPictureImportRGBA, WebPPreset, WebPValidateConfig,
    WEBP_CSP_MODE,
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
        Self { webp_data }
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

struct ManagedWebpAnim {
    anim_option: WebPAnimEncoderOptions,
    anim_encoder: *mut WebPAnimEncoder,
    webp_muxabi_ver: i32,
    frames: Vec<Frame>,
}

impl ManagedWebpAnim {
    fn new(frames: Vec<Frame>) -> Result<Self> {
        let first_frame = frames.first().context("cannot get first frame")?;
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

        Ok(Self {
            anim_option,
            webp_muxabi_ver: mux_abi_version,
            anim_encoder: encoder,
            frames,
        })
    }

    fn encode(self, quality_factor: f32) -> Result<Vec<u8>> {
        let mut time_stamp_ms = 0;
        for f in self.frames.iter() {
            self.anim_encoder_add(f, &mut time_stamp_ms, quality_factor)?;
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

    fn anim_encoder_add(
        &self,
        frame: &Frame,
        time_stamp: &mut u32,
        quality_factor: f32,
    ) -> Result<()> {
        let duration = frame.delay().numer_denom_ms();
        *time_stamp += duration.0 / duration.1;
        let mut pic = ManagedWebpPicture::from_rgba(frame.buffer(), quality_factor)?;
        let status = unsafe {
            WebPAnimEncoderAdd(
                self.anim_encoder,
                &mut pic.picture,
                *time_stamp as i32,
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

    fn check_mux_error(e: WebPMuxError) -> Result<()> {
        match e {
            WebPMuxError::WEBP_MUX_OK => Ok(()),
            _ => Err(anyhow::anyhow!("mux err")),
        }
    }
}

impl Drop for ManagedWebpAnim {
    fn drop(&mut self) {
        unsafe {
            WebPAnimEncoderDelete(self.anim_encoder);
        }
    }
}

/// アニメーションをWebpにエンコードする
pub(crate) fn encode_webp_anim(frames: Vec<Frame>, quality_factor: f32) -> Result<Vec<u8>> {
    let encoder = ManagedWebpAnim::new(frames)?;
    encoder.encode(quality_factor)
}

use libwebp_sys::{
    WebPAnimDecoder, WebPAnimDecoderDelete, WebPAnimDecoderGetInfo, WebPAnimDecoderGetNext,
    WebPAnimDecoderHasMoreFrames, WebPAnimDecoderNew, WebPAnimDecoderOptions,
    WebPAnimDecoderOptionsInit, WebPAnimDecoderReset, WebPAnimInfo,
};

struct ManagedWebpAnimDecoder<'a> {
    options: WebPAnimDecoderOptions,
    decoder: *mut WebPAnimDecoder,
    webp_data: WebPData,
    _ref: PhantomData<&'a [u8]>, // webp_dataが指している中身があることを保証する
}

impl<'a> ManagedWebpAnimDecoder<'a> {
    pub(crate) fn new(src: &'a [u8]) -> Result<Self> {
        let mut dec_options: WebPAnimDecoderOptions = unsafe { std::mem::zeroed() };
        let init_ok = unsafe { WebPAnimDecoderOptionsInit(&mut dec_options) };
        if init_ok != 1 {
            return Err(anyhow::anyhow!("anim decoder option init failed"));
        }

        dec_options.color_mode = WEBP_CSP_MODE::MODE_RGBA;

        let webp_data = WebPData {
            bytes: src.as_ptr(),
            size: src.len(),
        };

        let decoder = unsafe { WebPAnimDecoderNew(&webp_data, &dec_options) };

        if decoder.is_null() {
            return Err(anyhow::anyhow!("anim decoder init failed"));
        }

        Ok(Self {
            decoder,
            options: dec_options,
            webp_data,
            _ref: PhantomData,
        })
    }

    pub(crate) fn decode(&self) -> Result<Vec<Frame>> {
        let decoded = unsafe { self.decode_innternal()? };
        let mut frames = vec![];

        let mut before_timestamp = 0;
        for (buf, timestamp) in decoded {
            let delay = image::Delay::from_numer_denom_ms((timestamp - before_timestamp) as u32, 1);
            let f = Frame::from_parts(buf, 0, 0, delay);
            frames.push(f);
            before_timestamp = timestamp;
        }
        Ok(frames)
    }

    unsafe fn decode_innternal(&self) -> Result<Vec<(RgbaImage, i32)>> {
        let anim_info = self.get_anim_info()?;
        let width = anim_info.canvas_width;
        let height = anim_info.canvas_height;
        let outbuf_length = width * height * 4; // w * h * rgba
        let mut frames = vec![];
        while WebPAnimDecoderHasMoreFrames(self.decoder) > 0 {
            let mut outbuf = std::ptr::null_mut();
            let mut timestamp = 0;
            let is_ok = WebPAnimDecoderGetNext(self.decoder, &mut outbuf, &mut timestamp);
            if is_ok == 0 {
                return Err(anyhow::anyhow!("webp anim decode failed"));
            }
            let buf = std::slice::from_raw_parts(outbuf, outbuf_length as usize);
            let img = image::RgbaImage::from_raw(width, height, buf.to_vec())
                .context(anyhow::anyhow!("read rgba image failed"))?;
            frames.push((img, timestamp));
        }

        Ok(frames)
    }

    unsafe fn get_anim_info(&self) -> Result<WebPAnimInfo> {
        let mut anim_info: WebPAnimInfo = std::mem::zeroed();
        let info_ok = WebPAnimDecoderGetInfo(self.decoder, &mut anim_info);
        if info_ok != 1 {
            return Err(anyhow::anyhow!("getting anim info failed"));
        }
        Ok(anim_info)
    }

    pub(crate) fn count_frame(&self) -> Result<u32> {
        unsafe { self.get_anim_info().map(|x| x.frame_count) }
    }
}

impl<'a> Drop for ManagedWebpAnimDecoder<'a> {
    fn drop(&mut self) {
        unsafe {
            WebPAnimDecoderReset(self.decoder);
            WebPAnimDecoderDelete(self.decoder);
        }
    }
}

pub(crate) fn decode_webp_anim(src: &[u8]) -> Result<Vec<Frame>> {
    let decoder = ManagedWebpAnimDecoder::new(src)?;
    decoder.decode()
}
pub(crate) fn count_webp_anim_frame(src: &[u8]) -> Result<u32> {
    let decoder = ManagedWebpAnimDecoder::new(src)?;
    decoder.count_frame()
}

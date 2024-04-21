use anyhow::Result;
use image::{Frames, RgbaImage};
use libwebp_sys::WebPEncodeRGBA;

/// アニメーションを含まない画像をWebpにエンコードする
pub(crate) fn encode_webp_image(rgba_img: &RgbaImage) -> Result<Vec<u8>> {
    let mut out_buf = std::ptr::null_mut();
    let stride = rgba_img.width() * 4;
    let len = unsafe {
        WebPEncodeRGBA(
            rgba_img.as_raw().as_ptr(),
            rgba_img.width() as i32,
            rgba_img.height() as i32,
            stride as i32,
            75_f32,
            &mut out_buf,
        )
    };
    let out = unsafe { std::slice::from_raw_parts(out_buf, len) };
    Ok(out.into())
}

/// アニメーションをWebpにエンコードする
pub(crate) fn encode_webp_anim(frames: &Frames) -> Result<Vec<u8>> {
    todo!()
}

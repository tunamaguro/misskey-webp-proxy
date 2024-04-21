use image::{RgbaImage,Frames};
use anyhow::Result;

/// アニメーションを含まない画像をWebpにエンコードする
pub  (crate)fn encode_webp_image(rgba_img:&RgbaImage)->Result<Vec<u8>>{
    todo!()
}

/// アニメーションをWebpにエンコードする
pub(crate) fn encode_webp_anim(frames:&Frames)->Result<Vec<u8>>{
    todo!()
}
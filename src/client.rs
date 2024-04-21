use std::io::Cursor;

use anyhow::{Ok, Result};
use image::{AnimationDecoder, DynamicImage, Frames, RgbaImage};
use reqwest::{Client, Url};
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ImageExt {
    PNG,
    JPEG,
    GIF,
    SVG,
    WEBP,
    UNKNOWN,
}

pub enum DecodeResult<'a> {
    Image(RgbaImage),
    Movie(Frames<'a>),
    TextFmt(String),
}

/// 与えられたurlの画像拡張子を返す
pub(crate) fn get_image_ext(url: &Url) -> ImageExt {
    let p = url.path();
    match p.split('.').last() {
        Some("png") => ImageExt::PNG,
        Some("jpg") | Some("jpeg") | Some("jfif") | Some("pjpeg") | Some("pjp") => ImageExt::JPEG,
        Some("gif") => ImageExt::GIF,
        Some("svg") => ImageExt::SVG,
        Some("webp") => ImageExt::WEBP,
        _ => ImageExt::UNKNOWN,
    }
}

pub(crate) fn get_client(proxy_url: Option<&str>) -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();
    if let Some(url) = proxy_url {
        builder = builder.proxy(reqwest::Proxy::all(url)?);
    }
    let client = builder.build()?;
    Ok(client)
}

pub(crate) async fn download_image(client: Client, url: &Url) -> Result<DecodeResult> {
    let ext = get_image_ext(url);
    if ext == ImageExt::UNKNOWN {
        return Err(anyhow::anyhow!("Not supportted"));
    }
    let resp = client.get(url.clone()).send().await?;

    match ext {
        ImageExt::PNG => {
            let stream = Cursor::new(resp.bytes().await?);
            let decoder = image::codecs::png::PngDecoder::new(stream)?;
            let img = DynamicImage::from_decoder(decoder)?;
            Ok(DecodeResult::Image(img.to_rgba8()))
        }
        ImageExt::JPEG => {
            let stream = Cursor::new(resp.bytes().await?);
            let decoder = image::codecs::jpeg::JpegDecoder::new(stream)?;
            let img = DynamicImage::from_decoder(decoder)?;
            Ok(DecodeResult::Image(img.to_rgba8()))
        }
        ImageExt::GIF => {
            let stream = Cursor::new(resp.bytes().await?);
            let decoder = image::codecs::gif::GifDecoder::new(stream)?;
            let frames = decoder.into_frames();
            Ok(DecodeResult::Movie(frames))
        }
        ImageExt::SVG => {
            let txt = resp.text().await?;
            Ok(DecodeResult::TextFmt(txt))
        }
        ImageExt::WEBP => {
            let stream = Cursor::new(resp.bytes().await?);
            let decoder = image::codecs::webp::WebPDecoder::new(stream)?;
            match decoder.has_animation() {
                true => Ok(DecodeResult::Movie(decoder.into_frames())),
                false => {
                    let img = DynamicImage::from_decoder(decoder)?;
                    Ok(DecodeResult::Image(img.to_rgba8()))
                }
            }
        }
        ImageExt::UNKNOWN => Err(anyhow::anyhow!("Not supported")),
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use image::{Delay, Frame, ImageBuffer, ImageResult, Rgba};
    use pretty_assertions::assert_eq;
    use reqwest::Url;
    use rstest::rstest;

    // https://developer.mozilla.org/ja/docs/Web/Media/Formats/Image_types
    #[rstest]
    #[case("https://example.com/image.png", ImageExt::PNG)]
    #[case("https://example.com/image.jpg", ImageExt::JPEG)]
    #[case("https://example.com/image.jpeg", ImageExt::JPEG)]
    #[case("https://example.com/image.jfif", ImageExt::JPEG)]
    #[case("https://example.com/image.pjpeg", ImageExt::JPEG)]
    #[case("https://example.com/image.pjp", ImageExt::JPEG)]
    #[case("https://example.com/image.svg", ImageExt::SVG)]
    #[case("https://example.com/image.gif", ImageExt::GIF)]
    #[case("https://example.com/image.webp", ImageExt::WEBP)]
    #[case("https://example.com/image.apng", ImageExt::UNKNOWN)]
    #[case("https://example.com/image.avif", ImageExt::UNKNOWN)]
    #[case("https://example.com/image.bmp", ImageExt::UNKNOWN)]
    #[case("https://example.com/icon.ico", ImageExt::UNKNOWN)]
    #[case("https://example.com/icon.tiff", ImageExt::UNKNOWN)]
    #[case("https://example.com/hello", ImageExt::UNKNOWN)]
    #[case("https://example.com/hello.html", ImageExt::UNKNOWN)]
    #[case("https://example.com/", ImageExt::UNKNOWN)]
    fn parse_image_url(#[case] url: String, #[case] expected: ImageExt) {
        let url = Url::parse(&url).unwrap();
        assert_eq!(get_image_ext(&url), expected);
    }

    #[test]
    fn tmp() -> anyhow::Result<()> {
        use std::time::Duration;

        let width = 32u32;
        let height = 32u32;

        fn dumy_image(width: u32, height: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
            let mut imgbuf: ImageBuffer<image::Rgba<u8>, Vec<u8>> = ImageBuffer::new(width, height);
            for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
                let r = (0.3 * x as f32) as u8;
                let b = (0.3 * y as f32) as u8;
                *pixel = image::Rgba([r, 0, b, 0]);
            }
            imgbuf
        }

        let mut frames: Vec<Frame> = vec![];
        for _ in 0..100 {
            let duration = Duration::from_millis(20);
            let delay = Delay::from_saturating_duration(duration);
            frames.push(Frame::from_parts(dumy_image(width, height), 0, 0, delay))
        }

        let frames = Frames::new(Box::new(frames.into_iter().map(ImageResult::Ok)));
        let frames = frames.collect_frames()?;

        for f in frames {
            let d = f.delay().numer_denom_ms();
            println!("{:?}", d)
        }

        assert!(false);
        Ok(())
    }
}

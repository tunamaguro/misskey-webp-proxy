use std::{io::Cursor, net::IpAddr, str::FromStr};

use crate::{processor::DecodeResult, webp::decode_webp_anim};
use anyhow::Result;
use image::{AnimationDecoder, DynamicImage};
use reqwest::{Client, Url};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ImageExt {
    Png,
    Jpeg,
    Gif,
    Svg,
    Webp,
    Ico,
    Unknown,
}

/// 与えられたurlの画像拡張子を返す
/// https://developer.mozilla.org/en-US/docs/Web/Media/Formats/Image_types
pub(crate) fn get_image_ext(url: &Url) -> ImageExt {
    let p = url.path();
    match p.split('.').last() {
        Some("png") => ImageExt::Png,
        Some("jpg") | Some("jpeg") | Some("jfif") | Some("pjpeg") | Some("pjp") => ImageExt::Jpeg,
        Some("gif") => ImageExt::Gif,
        Some("svg") => ImageExt::Svg,
        Some("webp") => ImageExt::Webp,
        _ => ImageExt::Unknown,
    }
}

pub(crate) fn guess_format(buf: &[u8]) -> ImageExt {
    // 画像っぽいフォーマットの時の処理
    if let Ok(format) = image::guess_format(buf) {
        match format {
            image::ImageFormat::Png => {
                return ImageExt::Png;
            }
            image::ImageFormat::Jpeg => {
                return ImageExt::Jpeg;
            }
            image::ImageFormat::Gif => {
                return ImageExt::Gif;
            }
            image::ImageFormat::WebP => {
                return ImageExt::Webp;
            }
            image::ImageFormat::Pnm => {
                return ImageExt::Unknown;
            }
            image::ImageFormat::Tiff => {
                return ImageExt::Unknown;
            }
            image::ImageFormat::Tga => {
                return ImageExt::Unknown;
            }
            image::ImageFormat::Dds => {
                return ImageExt::Unknown;
            }
            image::ImageFormat::Bmp => {
                return ImageExt::Unknown;
            }
            image::ImageFormat::Ico => {
                return ImageExt::Ico;
            }
            image::ImageFormat::Hdr => {
                return ImageExt::Unknown;
            }
            image::ImageFormat::OpenExr => {
                return ImageExt::Unknown;
            }
            image::ImageFormat::Farbfeld => {
                return ImageExt::Unknown;
            }
            image::ImageFormat::Avif => {
                return ImageExt::Unknown;
            }
            image::ImageFormat::Qoi => {
                return ImageExt::Unknown;
            }
            _ => {}
        };
    }

    // それ以外の時はsvgとして処理を試みる
    ImageExt::Svg
}

pub(crate) fn get_client(proxy_url: Option<&str>) -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();
    if let Some(url) = proxy_url {
        builder = builder.proxy(reqwest::Proxy::all(url)?);
    }
    let client = builder.build()?;
    Ok(client)
}

/// ホストにIPアドレスを指定されているかチェックする  
/// TODO: グローバルに到達可能か検証する処理を追加する
fn is_private_like(url: &Url) -> bool {
    if let Some(host) = url.host() {
        return match host {
            url::Host::Domain(s) => IpAddr::from_str(s).is_ok(),
            url::Host::Ipv4(_) => true,
            url::Host::Ipv6(_) => true,
        };
    }
    true
}

pub(crate) async fn download_image(client: &Client, url: &Url) -> Result<DecodeResult> {
    if is_private_like(url) {
        return Err(anyhow::anyhow!("Cannot accept ipaddr"));
    }

    let resp = client.get(url.clone()).send().await?;
    let buf = resp.bytes().await?;
    let mut ext = get_image_ext(url);
    if ext == ImageExt::Unknown {
        ext = guess_format(&buf);
    }

    match ext {
        ImageExt::Png => {
            let stream = Cursor::new(buf);
            let decoder = image::codecs::png::PngDecoder::new(stream)?;
            let img = DynamicImage::from_decoder(decoder)?;
            Ok(DecodeResult::Image(img.to_rgba8()))
        }
        ImageExt::Jpeg => {
            let stream = Cursor::new(buf);
            let decoder = image::codecs::jpeg::JpegDecoder::new(stream)?;
            let img = DynamicImage::from_decoder(decoder)?;
            Ok(DecodeResult::Image(img.to_rgba8()))
        }
        ImageExt::Gif => {
            let stream = Cursor::new(buf);
            let decoder = image::codecs::gif::GifDecoder::new(stream)?;
            let frames = decoder.into_frames();
            Ok(DecodeResult::Movie(frames.collect_frames()?))
        }
        ImageExt::Svg => {
            let txt = String::from_utf8_lossy(&buf).to_string();
            Ok(DecodeResult::TextFmt(txt))
        }
        ImageExt::Webp => {
            let stream = Cursor::new(&buf);
            let decoder = image::codecs::webp::WebPDecoder::new(stream)?;

            match decoder.has_animation() {
                true => {
                    let frames = decode_webp_anim(&buf);
                    Ok(DecodeResult::Movie(frames?))
                }
                false => {
                    let img = DynamicImage::from_decoder(decoder)?;
                    Ok(DecodeResult::Image(img.to_rgba8()))
                }
            }
        }
        ImageExt::Ico => {
            let stream = Cursor::new(buf);
            let decoder = image::codecs::ico::IcoDecoder::new(stream)?;
            let img = DynamicImage::from_decoder(decoder)?;
            Ok(DecodeResult::Image(img.to_rgba8()))
        }
        ImageExt::Unknown => Err(anyhow::anyhow!("Not supported")),
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use pretty_assertions::assert_eq;
    use reqwest::Url;
    use rstest::rstest;

    // https://developer.mozilla.org/ja/docs/Web/Media/Formats/Image_types
    #[rstest]
    #[case("https://example.com/image.png", ImageExt::Png)]
    #[case("https://example.com/image.jpg", ImageExt::Jpeg)]
    #[case("https://example.com/image.jpeg", ImageExt::Jpeg)]
    #[case("https://example.com/image.jfif", ImageExt::Jpeg)]
    #[case("https://example.com/image.pjpeg", ImageExt::Jpeg)]
    #[case("https://example.com/image.pjp", ImageExt::Jpeg)]
    #[case("https://example.com/image.svg", ImageExt::Svg)]
    #[case("https://example.com/image.gif", ImageExt::Gif)]
    #[case("https://example.com/image.webp", ImageExt::Webp)]
    #[case("https://example.com/image.apng", ImageExt::Unknown)]
    #[case("https://example.com/image.avif", ImageExt::Unknown)]
    #[case("https://example.com/image.bmp", ImageExt::Unknown)]
    #[case("https://example.com/icon.ico", ImageExt::Unknown)]
    #[case("https://example.com/icon.tiff", ImageExt::Unknown)]
    #[case("https://example.com/hello", ImageExt::Unknown)]
    #[case("https://example.com/hello.html", ImageExt::Unknown)]
    #[case("https://example.com/", ImageExt::Unknown)]
    fn parse_image_url(#[case] url: String, #[case] expected: ImageExt) {
        let url = Url::parse(&url).unwrap();
        assert_eq!(get_image_ext(&url), expected);
    }
}

use crate::client::DecodeResult;
use anyhow::{Context, Result};
use image::{imageops, Frame, ImageResult};

use crate::webp::{encode_webp_anim, encode_webp_image};

/// 画像の変換処理を実装する
/// 仕様書: https://github.com/misskey-dev/media-proxy/blob/master/SPECIFICATION.md
impl<'a> DecodeResult<'a> {
    /// emojiを指定された際の大きさに変換する
    pub(crate) fn emoji(self) -> Result<DecodeResult<'a>> {
        const EMOJI_HEIGHT: u32 = 128;
        const EMOJI_WIDTH: u32 = 128;

        self.resize(EMOJI_HEIGHT, EMOJI_WIDTH)
    }

    /// avaterを指定された際の大きさに変換する
    pub(crate) fn avater(self) -> Result<DecodeResult<'a>> {
        const AVATER_HEIGHT: u32 = 320;
        const AVATER_WIDTH: u32 = 320;

        self.resize(AVATER_HEIGHT, AVATER_WIDTH)
    }

    /// previewを指定された際の大きさに変換する
    pub(crate) fn preview(self) -> Result<DecodeResult<'a>> {
        const PREVIEW_HEIGHT: u32 = 200;
        const PREVIEW_WIDTH: u32 = 200;

        self.resize(PREVIEW_HEIGHT, PREVIEW_WIDTH)
    }

    /// badgeに対応した際の大きさに変換する
    pub(crate) fn badge(self) -> Result<DecodeResult<'a>> {
        const BADGE_HEIGHT: u32 = 96;
        const BADGE_WIDTH: u32 = 96;

        self.resize(BADGE_HEIGHT, BADGE_WIDTH)
    }

    /// アニメーション画像であれば最初のフレームのみにする
    pub(crate) fn static_(self) -> Result<DecodeResult<'a>> {
        const STATIC_HEIGHT: u32 = 498;
        const STATIC_WIDTH: u32 = 422;

        self.first()?.resize(STATIC_HEIGHT, STATIC_WIDTH)
    }

    pub(crate) fn to_webp(self) -> Result<Vec<u8>> {
        match self {
            DecodeResult::Image(img) => encode_webp_image(img),
            DecodeResult::Movie(frames) => encode_webp_anim(frames),
            DecodeResult::TextFmt(_) => todo!("Not implemented"),
        }
    }

    /// 大きさを変換する
    fn resize(self, h: u32, w: u32) -> Result<DecodeResult<'a>> {
        match self {
            DecodeResult::Image(img) => {
                let resized = imageops::resize(&img, w, h, imageops::FilterType::Triangle);
                return Ok(DecodeResult::Image(resized));
            }
            DecodeResult::Movie(frames) => {
                let mut tmp = Vec::new();

                for f in frames {
                    let f = f?;
                    let resized =
                        imageops::resize(f.buffer(), w, h, imageops::FilterType::Triangle);
                    let new_frame = Frame::from_parts(resized, 0, 0, f.delay());
                    tmp.push(ImageResult::Ok(new_frame));
                }

                return Ok(DecodeResult::Movie(image::Frames::new(Box::new(
                    tmp.into_iter(),
                ))));
            }
            DecodeResult::TextFmt(_) => self.render_svg(h, w),
        }
    }

    /// svgを画像に変換する
    fn render_svg(self, _h: u32, _w: u32) -> Result<DecodeResult<'a>> {
        todo!("ここにsvgを画像にする処理を書く")
    }

    /// 一枚の画像に変換する。もとから単一の画像であれば何もしない
    fn first(self) -> Result<DecodeResult<'a>> {
        match self {
            DecodeResult::Image(_) => Ok(self),
            DecodeResult::TextFmt(_) => Ok(self),
            DecodeResult::Movie(mut frames) => {
                let first = frames.next().context("cannot find first frame")??;

                Ok(DecodeResult::Image(first.into_buffer()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::client::*;

    use anyhow::Ok;
    use reqwest::Url;
    use rstest::*;

    #[fixture]
    fn client() -> reqwest::Client {
        get_client(None).unwrap()
    }

    #[rstest]
    #[tokio::test]
    async fn webp_image_encode_test(client: reqwest::Client) -> anyhow::Result<()> {
        let url = Url::parse("https://github.com/tunamaguro.png")?;
        let res = download_image(client, &url).await?;
        let webp = res.to_webp()?;
        let mut file = tokio::fs::File::create("./tests/out/avater.webp").await?;

        let mut contents = Cursor::new(webp);
        tokio::io::copy(&mut contents, &mut file).await?;

        Ok(())
    }

    #[rstest]
    #[tokio::test]

    async fn webp_anim_encode_test(client: reqwest::Client) -> anyhow::Result<()> {
        // This gif image was created by Swfung8(https://commons.wikimedia.org/w/index.php?title=User:Swfung8&action=edit&redlink=1),CC BY 4.0
        let url = Url::parse(
            "https://upload.wikimedia.org/wikipedia/commons/9/9c/Insertion-sort-example.gif",
        )?;
        let res = download_image(client, &url).await?;
        match res {
            DecodeResult::Image(_) => todo!(),
            DecodeResult::Movie(_) => {
                println!("This is Movie")
            }
            DecodeResult::TextFmt(_) => todo!(),
        };

        let webp = res.to_webp()?;
        let mut file = tokio::fs::File::create("./tests/out/anim.webp").await?;

        let mut contents = Cursor::new(webp);
        tokio::io::copy(&mut contents, &mut file).await?;

        Ok(())
    }
}

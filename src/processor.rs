use anyhow::{Context, Ok, Result};
use image::{imageops, Frame, RgbaImage};

use crate::webp::{encode_webp_anim, encode_webp_image};

pub(crate) enum DecodeResult {
    Image(RgbaImage),
    Movie(Vec<Frame>),
    TextFmt(String),
}

/// 画像の変換処理を実装する
/// 仕様書: https://github.com/misskey-dev/media-proxy/blob/master/SPECIFICATION.md
impl DecodeResult {
    /// emojiを指定された際の大きさに変換する
    pub(crate) fn emoji(self) -> Result<DecodeResult> {
        const EMOJI_HEIGHT: u32 = 128;

        self.resize_by_height(EMOJI_HEIGHT)
    }

    /// avaterを指定された際の大きさに変換する
    pub(crate) fn avatar(self) -> Result<DecodeResult> {
        const AVATER_HEIGHT: u32 = 320;

        self.resize_by_height(AVATER_HEIGHT)
    }

    /// previewを指定された際の大きさに変換する
    pub(crate) fn preview(self) -> Result<DecodeResult> {
        const PREVIEW_HEIGHT: u32 = 200;
        const PREVIEW_WIDTH: u32 = 200;

        self.resize(PREVIEW_HEIGHT, PREVIEW_WIDTH)
    }

    /// badgeに対応した際の大きさに変換する
    pub(crate) fn badge(self) -> Result<DecodeResult> {
        const BADGE_HEIGHT: u32 = 96;
        const BADGE_WIDTH: u32 = 96;

        self.resize(BADGE_HEIGHT, BADGE_WIDTH)
    }

    /// アニメーション画像であれば最初のフレームのみにする。ついでに大きさも変換する
    pub(crate) fn static_(self) -> Result<DecodeResult> {
        const STATIC_HEIGHT: u32 = 422;

        self.first()?.resize_by_height(STATIC_HEIGHT)
    }

    pub(crate) fn to_webp(self, quality_factor: f32) -> Result<Vec<u8>> {
        match self {
            DecodeResult::Image(img) => encode_webp_image(img, quality_factor),
            DecodeResult::Movie(frames) => encode_webp_anim(frames, quality_factor),
            DecodeResult::TextFmt(_) => self.render_svg()?.to_webp(quality_factor),
        }
    }

    /// 大きさを変換する
    fn resize(self, h: u32, w: u32) -> Result<DecodeResult> {
        match self {
            DecodeResult::Image(img) => {
                let resized = imageops::resize(&img, w, h, imageops::FilterType::Triangle);
                Ok(DecodeResult::Image(resized))
            }
            DecodeResult::Movie(frames) => {
                let mut tmp = Vec::new();

                for f in frames {
                    let resized =
                        imageops::resize(f.buffer(), w, h, imageops::FilterType::Triangle);
                    let new_frame = Frame::from_parts(resized, 0, 0, f.delay());
                    tmp.push(new_frame);
                }

                Ok(DecodeResult::Movie(tmp))
            }
            DecodeResult::TextFmt(_) => self.render_svg()?.resize(h, w),
        }
    }

    /// 仕様書にあるように高さが`height`以下になるように変換を行う。その際アスペクト比は維持される
    /// ## Note
    /// もともとの画像もしくは動画の高さが`height`以下の場合何も行わない
    fn resize_by_height(self, height: u32) -> Result<Self> {
        let current_height = self.height()?;
        if current_height <= height {
            return Ok(self);
        }

        let width = self.width()? * height / current_height;
        self.resize(height, width)
    }

    /// svgを画像に変換する
    fn render_svg(self) -> Result<DecodeResult> {
        let res = match self {
            DecodeResult::Image(_) => self,
            DecodeResult::Movie(_) => self,
            DecodeResult::TextFmt(txt) => {
                let mut opt = usvg::Options::default();
                // opt.default_size = usvg::Size::from_wh(w as f32, h as f32).context("")?;
                let mut fontdb = usvg::fontdb::Database::new();
                fontdb.load_system_fonts();

                let tree = usvg::Tree::from_str(&txt, &opt, &fontdb)?;

                let pixmap_size: resvg::tiny_skia::IntSize = tree.size().to_int_size();
                let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height())
                    .context("init pixmap fail")?;
                resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

                let img =
                    RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.data().to_vec())
                        .context("render svg error")?;

                DecodeResult::Image(img)
            }
        };
        Ok(res)
    }

    /// 一枚の画像に変換する。もとから単一の画像であれば何もしない
    fn first(self) -> Result<DecodeResult> {
        match self {
            DecodeResult::Image(_) => Ok(self),
            DecodeResult::TextFmt(_) => Ok(self),
            DecodeResult::Movie(frames) => {
                let first = frames
                    .into_iter()
                    .next()
                    .context("cannot find first frame")?;

                Ok(DecodeResult::Image(first.into_buffer()))
            }
        }
    }

    /// 高さを返す。svgは未実装
    fn height(&self) -> Result<u32> {
        match self {
            DecodeResult::Image(img) => Ok(img.height()),
            DecodeResult::Movie(frames) => {
                let first = frames.first().context("cannot find first frame")?;
                Ok(first.buffer().height())
            }
            DecodeResult::TextFmt(txt) => {
                Ok(Self::create_svg_tree(txt)?.size().to_int_size().height())
            }
        }
    }

    /// 幅を返す。svgは未実装
    fn width(&self) -> Result<u32> {
        match self {
            DecodeResult::Image(img) => Ok(img.width()),
            DecodeResult::Movie(frames) => {
                let first = frames.first().context("cannot find first frame")?;
                Ok(first.buffer().width())
            }
            DecodeResult::TextFmt(txt) => {
                Ok(Self::create_svg_tree(txt)?.size().to_int_size().width())
            }
        }
    }

    fn create_svg_tree(txt: &String) -> Result<usvg::Tree> {
        let mut opt = usvg::Options::default();
        // opt.default_size = usvg::Size::from_wh(w as f32, h as f32).context("")?;
        let mut fontdb = usvg::fontdb::Database::new();
        fontdb.load_system_fonts();

        let tree = usvg::Tree::from_str(&txt, &opt, &fontdb)?;
        Ok(tree)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::{client::*, processor::DecodeResult};

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
        let res = download_image(&client, &url).await?;
        let webp = res.to_webp(75.0)?;
        let mut file = tokio::fs::File::create("./tests/out/avater.webp").await?;

        let mut contents = Cursor::new(webp);
        tokio::io::copy(&mut contents, &mut file).await?;

        Ok(())
    }

    #[rstest]
    #[tokio::test]

    async fn webp_anim_encode_test(client: reqwest::Client) -> anyhow::Result<()> {
        let url = Url::parse(
            "https://media1.giphy.com/media/v1.Y2lkPTc5MGI3NjExMG9laDA4MGFvb3FmaG1wZ3BjaGswYTNtM3hoc29jYmozbXl5d3d5MiZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/BfbUe877N4xsUhpcPc/giphy.gif",
        )?;
        let res = download_image(&client, &url).await?;

        let webp = res.to_webp(75.0)?;
        let mut file = tokio::fs::File::create("./tests/out/anim.webp").await?;

        let mut contents = Cursor::new(webp);
        tokio::io::copy(&mut contents, &mut file).await?;

        Ok(())
    }
}

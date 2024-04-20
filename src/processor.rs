use crate::client::DecodeResult;
use anyhow::Result;
use image::{imageops, Frame, ImageResult};

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

        todo!("ここに画像サイズを変更して、初期フレームを取り出す処理を書く")
    }

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
            DecodeResult::TextFmt(_) => todo!("ここにsvgを画像にする処理を書く"),
        }
    }
}

use crate::client::DecodeResult;
use image::imageops;


/// 画像の変換処理を実装する
/// 仕様書: https://github.com/misskey-dev/media-proxy/blob/master/SPECIFICATION.md
impl <'a> DecodeResult<'a> {
    /// emojiを指定された際の大きさに変換する
    pub(crate) fn emoji(self) -> DecodeResult<'a> {
        const EMOJI_HEIGHT: u32 = 128;
        const EMOJI_WIDTH: u32 = 128;

        todo!()
    }

    /// avaterを指定された際の大きさに変換する
    pub(crate) fn avater(self)-> DecodeResult<'a>{
        const AVATER_HEIGHT:u32 = 320;
        const AVATER_WIDTH:u32 = 320;

        todo!()
    }

    /// previewを指定された際の大きさに変換する
    pub(crate) fn preview(self)->DecodeResult<'a>{
        const PREVIEW_HEIGHT:u32 = 200;
        const PREVIEW_WIDTH:u32 = 200;

        todo!()
    }

    /// badgeに対応した際の大きさに変換する
    pub(crate) fn badge(self)->DecodeResult<'a>{
        const BADGE_HEIGHT:u32 = 96;
        const BADGE_WIDTH:u32 = 96;

        todo!()
    }

    /// アニメーション画像であれば最初のフレームのみにする
    pub(crate) fn static_(self)->DecodeResult<'a>{
        const STATIC_HEIGHT:u32 = 498;
        const STATIC_WIDTH:u32 = 422;

        todo!()
    }
}

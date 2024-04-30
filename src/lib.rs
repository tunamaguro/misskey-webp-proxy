mod client;
mod processor;
mod webp;

use anyhow::Result;
use client::{download_image, get_image_ext, ImageExt};
use reqwest::{Client, Url};
use serde::Deserialize;

#[derive(Debug, PartialEq, Deserialize)]
/// メディアプロキシのクエリ。フォールバックには未対応
struct ProxyQuery {
    url: String,
    emoji: Option<usize>,
    avatar: Option<usize>,
    r#static: Option<usize>,
    preview: Option<usize>,
    badge: Option<usize>,
}

#[derive(Debug, PartialEq)]
enum ConvertType {
    EMOJI,
    AVATAR,
    PREVIEW,
    BADGE,
    ORIGINAL,
}

#[derive(Debug, PartialEq)]
struct ProxyConfig {
    url: Url,
    convert_type: ConvertType,
    is_static: bool,
}

impl TryFrom<ProxyQuery> for ProxyConfig {
    type Error = anyhow::Error;

    fn try_from(value: ProxyQuery) -> Result<Self, Self::Error> {
        let url = Url::parse(&value.url)?;
        let convert_type: ConvertType = if value.emoji.is_some() {
            ConvertType::EMOJI
        } else if value.avatar.is_some() {
            ConvertType::AVATAR
        } else if value.preview.is_some() {
            ConvertType::PREVIEW
        } else if value.badge.is_some() {
            ConvertType::BADGE
        } else {
            ConvertType::ORIGINAL
        };

        let is_static = value.r#static.is_some();
        Ok({
            ProxyConfig {
                url,
                convert_type,
                is_static,
            }
        })
    }
}

async fn media_proxy(client: &Client, config: &ProxyConfig) -> Result<Vec<u8>> {
    let mut decoded_buf = download_image(client, &config.url).await?;
    match config.is_static {
        true => decoded_buf = decoded_buf.static_()?,
        false => {
            // do nothing
        }
    }

    match config.convert_type {
        ConvertType::EMOJI => decoded_buf = decoded_buf.emoji()?,
        ConvertType::AVATAR => decoded_buf = decoded_buf.avater()?,
        ConvertType::PREVIEW => decoded_buf = decoded_buf.preview()?,
        ConvertType::BADGE => decoded_buf = decoded_buf.badge()?,
        ConvertType::ORIGINAL => {
            // do nothing
        }
    }

    decoded_buf.to_webp()
}

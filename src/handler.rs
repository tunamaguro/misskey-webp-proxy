use crate::{client::download_image, processor::DecodeResult};
use anyhow::{Ok, Result};
use reqwest::{Client, Url};
use serde::Deserialize;

/// メディアプロキシのクエリ。フォールバックには未対応
#[derive(Debug, PartialEq, Deserialize)]
pub(crate) struct ProxyQuery {
    url: String,
    emoji: Option<usize>,
    avatar: Option<usize>,
    r#static: Option<usize>,
    preview: Option<usize>,
    badge: Option<usize>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum ConvertType {
    Emoji,
    Avatar,
    Preview,
    Badge,
    Original,
}

#[derive(Debug, PartialEq)]
pub(crate) struct ProxyConfig {
    pub(crate) url: Url,
    pub(crate) convert_type: ConvertType,
    pub(crate) is_static: bool,
}

impl TryFrom<ProxyQuery> for ProxyConfig {
    type Error = anyhow::Error;

    fn try_from(value: ProxyQuery) -> Result<Self, Self::Error> {
        let url = Url::parse(&value.url)?;
        let convert_type: ConvertType = if value.emoji.is_some() {
            ConvertType::Emoji
        } else if value.avatar.is_some() {
            ConvertType::Avatar
        } else if value.preview.is_some() {
            ConvertType::Preview
        } else if value.badge.is_some() {
            ConvertType::Badge
        } else {
            ConvertType::Original
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

pub(crate) async fn media_proxy(
    client: &Client,
    proxy_config: &ProxyConfig,
) -> Result<DecodeResult> {
    let mut decoded_buf = download_image(client, &proxy_config.url).await?;
    match proxy_config.is_static {
        true => decoded_buf = decoded_buf.static_()?,
        false => {
            // do nothing
        }
    }

    match proxy_config.convert_type {
        ConvertType::Emoji => decoded_buf = decoded_buf.emoji()?,
        ConvertType::Avatar => decoded_buf = decoded_buf.avatar()?,
        ConvertType::Preview => decoded_buf = decoded_buf.preview()?,
        ConvertType::Badge => decoded_buf = decoded_buf.badge()?,
        ConvertType::Original => {
            // do nothing
        }
    }

    Ok(decoded_buf)
}

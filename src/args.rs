use clap::Parser;
use http::HeaderValue;

#[derive(Parser, Debug)]
#[command(
    version,
    about,
    long_about = "Misskey Media Proxyの実装です。現在開発段階のため、趣味以外で使うことはお勧めしません"
)]
pub(crate) struct Args {
    #[arg(
        long,
        env,
        help = "Media Proxyが利用するhttp proxyです。設定しない場合http proxyを利用しません"
    )]
    pub(crate) http_proxy: Option<String>,
    #[arg(
        long,
        env,
        default_value_t = 3000,
        help = "Media Proxyが待機するポートです"
    )]
    pub(crate) port: u32,
    #[arg(
        long,
        env,
        default_value = "0.0.0.0",
        help = "Media Proxyが待機するアドレスです"
    )]
    pub(crate) host: String,
    #[arg(
        long,
        default_value_t = 75,
        help = "Webpの圧縮率です。0-100の範囲で指定でき、0が最も高い圧縮率ですが画質が低くなります"
    )]
    pub(crate) quality_factor: u8,
    #[arg(
        long,
        help = "CORSの設定です。未設定の場合、すべてのオリジンからのリクエストを受け付けます"
    )]
    pub(crate) allow_origin: Vec<String>,
}

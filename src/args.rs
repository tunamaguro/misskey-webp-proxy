use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = "Misskey Media Proxyの実装です。現在開発段階のため、趣味以外で使うことはお勧めしません")]
pub(crate) struct Args {
    #[arg(long,help="Media Proxyが利用するhttp proxyです。設定しない場合http proxyを利用しません")]
    pub(crate) http_proxy: Option<String>
}

pub mod client {
    use anyhow::Result;
    use reqwest::Url;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub(crate) enum ImageExt {
        PNG,
        JPEG,
        GIF,
        SVG,
        WEBP,
        UNKNOWN,
    }

    /// 与えられたurlの画像拡張子を返す
    pub(crate) fn get_image_ext(url: &Url) -> ImageExt {
        let p = url.path();
        match p.split(".").last() {
            Some("png") => ImageExt::PNG,
            Some("jpg") | Some("jpeg") | Some("jfif") | Some("pjpeg") | Some("pjp") => {
                ImageExt::JPEG
            }
            Some("gif") => ImageExt::GIF,
            Some("svg") => ImageExt::SVG,
            Some("webp") => ImageExt::WEBP,
            _ => ImageExt::UNKNOWN,
        }
    }

    pub(crate) async fn download_image(url: Url) -> Result<()> {
        let resp = reqwest::get(url).await?;

        return Ok(());
    }
}

#[cfg(test)]
mod tests {

    use super::client::*;
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
}

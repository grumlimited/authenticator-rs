use regex::Regex;
use scraper::*;
use std::sync::mpsc::Sender;
#[derive(Debug, Clone)]
pub struct IconParser {}

pub type IconParserResult<T> = std::result::Result<T, IconError>;

#[derive(Debug)]
pub enum IconError {
    ParsingError,
    ReqwestError(reqwest::Error),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccountGroupIcon {
    content: Vec<u8>,
    extension: Option<String>,
}

impl IconParser {
    async fn html(
        sender: Sender<AccountGroupIcon>,
        url: &str,
    ) -> IconParserResult<AccountGroupIcon> {
        let response = reqwest::get(url).await.map_err(IconError::ReqwestError)?;
        let html = response.text().await.map_err(IconError::ReqwestError)?;
        Self::icon(sender, url, html.as_str()).await
    }

    async fn icon(
        sender: Sender<AccountGroupIcon>,
        url: &str,
        html: &str,
    ) -> IconParserResult<AccountGroupIcon> {
        let icon_url: std::result::Result<String, IconError> = {
            let document = Html::parse_document(html);

            let selector = Selector::parse(r#"link[rel="apple-touch-icon"]"#).unwrap();

            match document
                .select(&selector)
                .into_iter()
                .next()
                .and_then(|v| v.value().attr("href"))
            {
                Some(href) if href.starts_with("/") => Ok(format!("{}/{}", url, href)),
                Some(href) if href.starts_with("http") => Ok(format!("{}", href)),
                Some(href) => Ok(format!("{}/{}", url, href)),
                None => Err(IconError::ParsingError),
            }
        };

        match icon_url {
            Ok(icon_url) => Self::download(sender, icon_url.as_str()).await,
            Err(r) => Err(r),
        }
    }

    async fn download(
        sender: Sender<AccountGroupIcon>,
        icon_url: &str,
    ) -> IconParserResult<AccountGroupIcon> {
        let response = reqwest::get(icon_url)
            .await
            .map_err(IconError::ReqwestError)?;
        let content_type = response.headers().get("content-type");

        let extension = content_type.and_then(|content_type| match content_type.to_str() {
            Ok(content_type) => Self::extension(content_type).map(str::to_owned),
            Err(_) => None,
        });

        let bytes = response.bytes().await.map_err(IconError::ReqwestError)?;

        let result = AccountGroupIcon {
            content: bytes.to_vec(),
            extension,
        };

        sender.send(result.clone()).expect("Boom!");

        Ok(result)
    }

    fn extension(content_type: &str) -> Option<&str> {
        let regex = Regex::new(r"^.*/(?P<extension>.*?)$").unwrap();

        regex.captures(content_type).and_then(|captures| {
            captures
                .name("extension")
                .map(|extension| extension.as_str())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn extension() {
        assert_eq!("png", IconParser::extension("image/png").unwrap());
        assert_eq!(None, IconParser::extension(""));
        assert_eq!(None, IconParser::extension("no slash"));
    }

    #[test]
    fn download() {
        let mut rt = tokio::runtime::Builder::new()
            .threaded_scheduler()
            .enable_all()
            .build()
            .unwrap();

        let (sender, receiver) = channel::<AccountGroupIcon>();

        let fut = IconParser::download(
            sender,
            "https://static.bbci.co.uk/wwhp/1.145.0/responsive/img/apple-touch/apple-touch-180.jpg",
        );

        let icon_parser_result = rt.block_on(rt.spawn(fut)).unwrap().unwrap();
        assert_eq!("jpeg", icon_parser_result.extension.unwrap());

        let icon_parser_result = receiver.recv().unwrap();
        assert_eq!("jpeg", icon_parser_result.extension.unwrap());
    }

    #[test]
    fn html() {
        let mut rt = tokio::runtime::Builder::new()
            .threaded_scheduler()
            .enable_all()
            .build()
            .unwrap();

        let (sender, receiver) = channel::<AccountGroupIcon>();

        let fut = IconParser::html(sender, "https://www.bbc.com");

        let icon_parser_result = rt.block_on(rt.spawn(fut)).unwrap().unwrap();
        assert_eq!("jpeg", icon_parser_result.extension.unwrap());

        let icon_parser_result = receiver.recv().unwrap();
        assert_eq!("jpeg", icon_parser_result.extension.unwrap());
    }
}

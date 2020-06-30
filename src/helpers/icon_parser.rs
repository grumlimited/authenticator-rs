use regex::Regex;
use reqwest::*;
use scraper::*;
use std::sync::mpsc::{channel, Sender};
#[derive(Debug, Clone)]
pub struct IconParser {}
use crate::helpers::LoadError::SaveError;
use reqwest::header::HeaderValue;
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq)]
pub enum IconError {
    ParsingError,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IconParserResult {
    content: Vec<u8>,
    extension: Option<String>,
}

impl IconParser {
    async fn html(sender: Sender<IconParserResult>, url: &str) -> Result<IconParserResult> {
        let response = reqwest::get(url).await?;
        let html = response.text().await?;
        Self::icon(sender, url, html.as_str()).await
    }

    async fn icon(
        sender: Sender<IconParserResult>,
        url: &str,
        html: &str,
    ) -> Result<IconParserResult> {
        let icon_url: std::result::Result<String, IconError> = {
            let document = Html::parse_document(html);

            let selector = Selector::parse(r#"link[rel="icon"]"#).unwrap();

            match document
                .select(&selector)
                .into_iter()
                .next()
                .and_then(|v| v.value().attr("href"))
            {
                Some(href) => Ok(format!("{}/{}", url, href)),
                None => Err(IconError::ParsingError),
            }
        };

        Self::download(sender, icon_url.unwrap().as_str()).await
    }

    async fn download(
        sender: Sender<IconParserResult>,
        icon_url: &str,
    ) -> Result<IconParserResult> {
        let response = reqwest::get(icon_url).await?;
        let content_type = response.headers().get("content-type");

        let extension = content_type.and_then(|content_type| match content_type.to_str() {
            Ok(content_type) => Self::extension(content_type).map(str::to_owned),
            Err(_) => None,
        });

        let bytes = response.bytes().await?;

        let result = IconParserResult {
            content: bytes.to_vec(),
            extension,
        };

        sender.send(result.clone());

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
    use async_std::task;
    use tokio::prelude::*;
    use tokio::runtime::Runtime;

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

        let (sender, receiver) = channel::<IconParserResult>();

        let fut = IconParser::download(sender, "https://www.rust-lang.org/static/images/favicon-32x32.png");

        let icon_parser_result = rt.block_on(rt.spawn(fut)).unwrap().unwrap();
        assert_eq!("png", icon_parser_result.extension.unwrap());

        let icon_parser_result = receiver.recv().unwrap();
        assert_eq!("png", icon_parser_result.extension.unwrap());
    }

    #[test]
    fn xxx() {
        let mut rt = tokio::runtime::Builder::new()
            .threaded_scheduler()
            .enable_all()
            .build()
            .unwrap();

        let (sender, receiver) = channel::<IconParserResult>();

        let r: tokio::task::JoinHandle<_> =
            rt.spawn(IconParser::html(sender, "https://www.rust-lang.org"));
        rt.block_on(r);

        // let s = receiver.recv();

        // println!("{:?}", s)

        // let x1 = s.unwrap();
        // // let html = x1.unwrap();
        // let x = x1.to_owned();
        // let html = x.as_str();
        //
        // let r: tokio::task::JoinHandle<_> = rt.spawn(IconParser::icon(html));
        // let s = rt.block_on(r);
        // println!("{:?}", s)
    }
}

use reqwest::*;
use scraper::*;
use std::sync::mpsc::{channel, Sender};

#[derive(Debug, Clone)]
pub struct IconParser {}

#[derive(Debug, Clone, PartialEq)]
pub enum IconError {
    ParsingError,
}

impl IconParser {
    // #[tokio::main]
    async fn html(sender: Sender<String>, url: &str) -> Result<()> {
        // let html: Result<String> = match reqwest::get(url).await {
        //     Ok(response) => response.text().await,
        //     Err(e) => Err(e),
        // };
        //
        // match html {
        //     Ok(html) => {
        //         Self::icon(sender, url, html.as_str()).await;
        //     }
        //     Err(e) => {}
        // };

        let html = reqwest::get(url).await?.text().await?;
        Self::icon(sender, url, html.as_str()).await
    }

    async fn icon(sender: Sender<String>, url: &str, html: &str) -> Result<()> {
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

    async fn download(sender: Sender<String>, icon_url: &str) -> Result<()> {
        // let mut resp = reqwest::get(icon_url).await;
        //
        // match resp {
        //     Ok(response) => {
        //         let bytes: Result<bytes::Bytes> = response.bytes().await;
        //         let bytes = bytes.unwrap();
        //         std::fs::write("/tmp/foo", bytes.to_vec()).expect("Unable to write file");
        //     }
        //     Err(e) => {}
        // };

        let bytes = reqwest::get(icon_url).await?.bytes().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std::task;
    use tokio::prelude::*;
    use tokio::runtime::Runtime;

    #[test]
    fn xxx() {
        let mut rt = tokio::runtime::Builder::new()
            .threaded_scheduler()
            .enable_all()
            .build()
            .unwrap();

        let (sender, receiver) = channel::<String>();

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

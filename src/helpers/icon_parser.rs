use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use curl::easy::Easy;
use gdk_pixbuf::Pixbuf;
use glib::Sender;
use log::debug;
use regex::Regex;
use scraper::*;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct IconParser {}

#[derive(Debug, Error)]
pub enum IconError {
    #[error("Could not find icon in html")]
    ParsingError,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AccountGroupIcon {
    pub content: Vec<u8>,
    pub extension: Option<String>,
}

impl IconParser {
    pub async fn html_notify(sender: Sender<Result<AccountGroupIcon>>, url: String) {
        let result = Self::html(&url).await;
        sender.send(result).expect("Could not send results");
    }

    pub async fn html(url: &str) -> Result<AccountGroupIcon> {
        let (data, extension) = Self::download(url).await?;

        match extension {
            Some(extension) if extension.ends_with("icon") => Ok(AccountGroupIcon {
                content: data,
                extension: Some(extension),
            }),
            _ => {
                let html = String::from_utf8_lossy(data.as_slice()).into_owned();
                Self::icon(url, html.as_str()).await
            }
        }
    }

    async fn icon(url: &str, html: &str) -> Result<AccountGroupIcon> {
        let icon_url = {
            let document = Html::parse_document(html);

            let selector_1 = Selector::parse(r#"link[rel="apple-touch-icon"]"#).unwrap();
            let selector_2 = Selector::parse(r#"link[rel="shortcut icon"]"#).unwrap();
            let selector_3 = Selector::parse(r#"link[rel="icon"]"#).unwrap();

            let option_1 = document.select(&selector_1).next();
            let option_2 = document.select(&selector_2).next();
            let option_3 = document.select(&selector_3).next();

            let choice = option_1.or(option_2).or(option_3);

            match choice.and_then(|v| v.value().attr("href")) {
                Some(href) if href.starts_with('/') => Ok(format!("{}/{}", url, href)),
                Some(href) if href.starts_with("http") => Ok(href.to_string()),
                Some(href) => Ok(format!("{}/{}", url, href)),
                None => Err(IconError::ParsingError),
            }
        }?;

        debug!("icon_url: {}", icon_url);

        Self::download_icon(icon_url.as_str()).await
    }

    async fn download_icon(icon_url: &str) -> Result<AccountGroupIcon> {
        let (data, extension) = Self::download(icon_url).await?;
        Ok(AccountGroupIcon { content: data, extension })
    }

    async fn download(icon_url: &str) -> Result<(Vec<u8>, Option<String>)> {
        let mut data = Vec::new();
        let mut handle = Easy::new();

        handle.follow_location(true)?;
        handle.useragent("Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/113.0")?;
        handle.autoreferer(true)?;
        handle.timeout(Duration::from_secs(5))?;
        handle.url(icon_url)?;

        {
            let mut transfer = handle.transfer();
            transfer.write_function(|new_data| {
                data.extend_from_slice(new_data);
                Ok(new_data.len())
            })?;

            transfer.perform()?;
        }

        let extension = handle.content_type().map(|e| e.and_then(Self::extension).map(str::to_owned))?;

        Ok((data, extension))
    }

    fn extension(content_type: &str) -> Option<&str> {
        let regex = Regex::new(r"^.*/(?P<extension>.*?)$").unwrap();

        regex
            .captures(content_type)
            .and_then(|captures| captures.name("extension").map(|extension| extension.as_str()))
    }

    pub fn load_icon(filepath: &Path, dark_mode: bool) -> Result<Pixbuf> {
        let alpha = if dark_mode { (32, 32, 32) } else { (255, 255, 255) };

        debug!("loading icon {} with alpha channels {:?}", filepath.display(), &alpha);

        Pixbuf::from_file_at_scale(filepath, 48, 48, true)
            .map_err(|e| e.into())
            .and_then(|pixbuf| pixbuf.add_alpha(true, alpha.0, alpha.1, alpha.2).map_err(|e| e.into()))
    }
}

#[cfg(test)]
mod tests {
    use async_std::task;

    use super::*;

    #[test]
    fn extension() {
        assert_eq!("png", IconParser::extension("image/png").unwrap());
        assert_eq!(None, IconParser::extension(""));
        assert_eq!(None, IconParser::extension("no slash"));
    }

    #[test]
    fn download() {
        let fut = IconParser::download_icon("https://static.bbci.co.uk/wwhp/1.145.0/responsive/img/apple-touch/apple-touch-180.jpg");

        let icon_parser_result = task::block_on(fut).unwrap();
        assert_eq!("jpeg", icon_parser_result.extension.unwrap());
    }

    #[test]
    fn html() {
        let fut = IconParser::html("https://www.bbc.com");

        let icon_parser_result = task::block_on(fut).unwrap();
        assert_eq!("jpeg", icon_parser_result.extension.unwrap());
    }
}

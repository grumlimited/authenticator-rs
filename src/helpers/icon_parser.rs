use curl::easy::Easy;
use curl::Error;
use futures::AsyncWriteExt;
use gdk::enums::key::ht;
use glib::Sender;
use regex::Regex;
use scraper::*;

#[derive(Debug, Clone)]
pub struct IconParser {}

pub type IconParserResult<T> = std::result::Result<T, IconError>;

#[derive(Debug)]
pub enum IconError {
    ParsingError,
    CurlError(Error),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccountGroupIcon {
    pub content: Vec<u8>,
    pub extension: Option<String>,
}

impl IconParser {
    pub async fn html_notify(sender: Sender<IconParserResult<AccountGroupIcon>>, url: String) {
        let result = Self::html(url).await;
        sender.send(result);
    }

    pub async fn html(url: String) -> IconParserResult<AccountGroupIcon> {
        let mut data = Vec::new();

        let mut handle = Easy::new();
        handle.follow_location(true);
        handle.autoreferer(true);
        handle.url(url.as_str()).unwrap();

        {
            let mut transfer = handle.transfer();
            transfer
                .write_function(|new_data| {
                    data.extend_from_slice(new_data);
                    Ok(new_data.len())
                })
                .map_err(IconError::CurlError)?;
            transfer.perform().map_err(IconError::CurlError)?;
        }

        let html = String::from_utf8_lossy(data.as_slice()).into_owned();

        Self::icon(url.as_str(), html.as_str()).await
    }

    async fn icon(url: &str, html: &str) -> IconParserResult<AccountGroupIcon> {
        let icon_url = {
            //println!("{}", html);
            let document = Html::parse_document(html);

            let selector_1 = Selector::parse(r#"link[rel="icon"]"#).unwrap();
            let selector_2 = Selector::parse(r#"link[rel="apple-touch-icon"]"#).unwrap();

            let option_1 = document.select(&selector_1).into_iter().next();

            let option_2 = document.select(&selector_2).into_iter().next();

            let choice = option_1.or(option_2);

            match choice.and_then(|v| v.value().attr("href")) {
                Some(href) if href.starts_with("/") => Ok(format!("{}/{}", url, href)),
                Some(href) if href.starts_with("http") => Ok(format!("{}", href)),
                Some(href) => Ok(format!("{}/{}", url, href)),
                None => Err(IconError::ParsingError),
            }
        }?;

        println!("{}", icon_url);

        Self::download(icon_url.as_str()).await
    }

    async fn download(icon_url: &str) -> IconParserResult<AccountGroupIcon> {
        let mut data = Vec::new();
        let mut handle = Easy::new();

        handle.follow_location(true);
        handle.autoreferer(true);
        handle.url(icon_url).map_err(IconError::CurlError)?;

        {
            let mut transfer = handle.transfer();
            transfer
                .write_function(|new_data| {
                    data.extend_from_slice(new_data);
                    Ok(new_data.len())
                })
                .map_err(IconError::CurlError)?;

            transfer.perform().map_err(IconError::CurlError)?;
        }

        let extension = handle
            .content_type()
            .map_err(IconError::CurlError)
            .map(|e| e.and_then(Self::extension).map(str::to_owned))?;

        Ok(AccountGroupIcon {
            content: data,
            extension,
        })
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

    #[test]
    fn extension() {
        assert_eq!("png", IconParser::extension("image/png").unwrap());
        assert_eq!(None, IconParser::extension(""));
        assert_eq!(None, IconParser::extension("no slash"));
    }

    #[test]
    fn download() {
        let fut = IconParser::download(
            "https://static.bbci.co.uk/wwhp/1.145.0/responsive/img/apple-touch/apple-touch-180.jpg",
        );

        let icon_parser_result = task::block_on(fut).unwrap();
        assert_eq!("jpeg", icon_parser_result.extension.unwrap());
    }

    #[test]
    fn html() {
        let fut = IconParser::html("https://www.bbc.co.uk".to_owned());

        let icon_parser_result = task::block_on(fut).unwrap();
        assert_eq!("png", icon_parser_result.extension.unwrap());
    }
}

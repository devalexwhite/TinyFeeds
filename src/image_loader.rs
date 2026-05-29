use std::sync::LazyLock;
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct Image {
    pub bytes: Vec<u8>,
    #[allow(dead_code)]
    pub url: String,
    #[allow(dead_code)]
    pub is_svg: bool,
}

pub async fn download_image(url: String, base_url: Option<String>) -> Result<Image, String> {
    static CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

    let resolved_url = if let Some(base_str) = base_url {
        if let Ok(base_url) = reqwest::Url::parse(&base_str) {
            match base_url.join(&url) {
                Ok(joined) => joined.to_string(),
                Err(_) => url.clone(),
            }
        } else {
            url.clone()
        }
    } else {
        url.clone()
    };

    let response = CLIENT
        .get(&resolved_url)
        .send()
        .await
        .map_err(|err| err.to_string())?;

    if !response.status().is_success() {
        Err(format!("Error {} from url: {resolved_url}", response.status()))
    } else {
        let bytes = response
            .bytes()
            .await
            .map_err(|err| err.to_string())?
            .to_vec();
        Ok(Image {
            is_svg: bytes.starts_with(b"<svg "),
            url,
            bytes,
        })
    }
}

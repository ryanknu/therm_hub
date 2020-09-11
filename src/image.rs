#[cfg(not(any(test, feature = "offline")))]
use serde::Deserialize;
#[cfg(not(any(test, feature = "offline")))]
use serde_json::Value;
#[cfg(not(any(test, feature = "offline")))]
use std::path::Path;
use std::path::PathBuf;

// TODO: Make this file a lot more safe

#[cfg(not(any(test, feature = "offline")))]
#[derive(Deserialize)]
struct WebStream {
    photos: Vec<Photo>,
}

#[cfg(not(any(test, feature = "offline")))]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Photo {
    photo_guid: String,
}

#[cfg(not(any(test, feature = "offline")))]
#[derive(Deserialize)]
struct WebAssetUrls {
    items: Value,
}

#[cfg(not(any(test, feature = "offline")))]
async fn download_and_store(url: &str, file_name: &str) -> anyhow::Result<()> {
    let path = format!("{}/{}.jpg", std::env::var("PHOTO_CACHE_DIR")?, file_name);
    let path = Path::new(&path);

    if path.is_file() {
        println!("Skipping {} because it is already on disk", file_name);
        return Ok(());
    }

    let bytes = crate::REQWEST.get(url).send().await?.bytes().await?;

    let buffer = image::load_from_memory(&*bytes)?;

    // Only save full size images. Skip thumbnails.
    if buffer.to_rgb().width() > 800 {
        let buffer = buffer.resize_to_fill(800, 480, image::imageops::FilterType::Lanczos3);
        buffer.save(path)?;
        println!("Downloaded image to {}", file_name);
    } else {
        println!("Skipping {} because it is a thumbnail", file_name);
    }

    Ok(())
}

#[tokio::main]
#[cfg(any(test, feature = "offline"))]
pub async fn scrape_webstream() -> anyhow::Result<()> {
    Ok(())
}

#[tokio::main]
#[cfg(not(any(test, feature = "offline")))]
pub async fn scrape_webstream() -> anyhow::Result<()> {
    let album_id = std::env::var("SHARED_ALBUM_ID")?;
    let data = crate::REQWEST
        .post(&format!(
            "https://p26-sharedstreams.icloud.com/{}/sharedstreams/webstream",
            album_id
        ))
        .header("User-Agent", "github.com/ryanknu/therm_hub")
        .body("{\"streamCtag\":null}")
        .send()
        .await?
        .text()
        .await?;

    let web_stream: WebStream = serde_json::from_str(&data)?;

    let guids: Vec<String> = web_stream
        .photos
        .into_iter()
        .map(|x| x.photo_guid)
        .collect();

    let body = format!("{{\"photoGuids\": {}}}", serde_json::to_string(&guids)?);

    let data = crate::REQWEST
        .post(&format!(
            "https://p26-sharedstreams.icloud.com/{}/sharedstreams/webasseturls",
            album_id
        ))
        .header("User-Agent", "github.com/ryanknu/therm_hub")
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await?
        .text()
        .await?;

    let map: WebAssetUrls = serde_json::from_str(&data)?;
    let map = map.items.as_object().unwrap();

    for (_, image) in map.into_iter() {
        let image = image.as_object().unwrap();
        let host: String = image
            .get("url_location")
            .unwrap()
            .to_string()
            .replace("\"", "");
        let path: String = image.get("url_path").unwrap().to_string().replace("\"", "");
        let file_name: String = String::from(&path[4..31]);
        let url = format!("https://{}{}", host, path);
        let _ = download_and_store(&url, &file_name).await;
    }

    Ok(())
}

pub fn photo_paths() -> Vec<PathBuf> {
    let pattern = format!(
        "{}/*.jpg",
        std::env::var("PHOTO_CACHE_DIR").expect("PHOTO_CACHE_DIR not set!")
    );
    glob::glob(&pattern)
        .expect("Failed to read glob pattern")
        .filter_map(|path| match path {
            Ok(path) => Some(path),
            _ => None,
        })
        .collect()
}

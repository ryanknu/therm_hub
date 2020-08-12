use serde::Deserialize;
// Below is my little HTTP library

// TODO: replace option with my error class
pub fn parse<'de, T>(data: &'de str) -> Option<T>
    where T: Deserialize<'de> {
    let result: Result<T, serde_json::error::Error> = serde_json::from_str(&data);
    match result {
        Ok(data) => Some(data),
        Err(err) => {
            println!("[ hyper] JSON: {}", data);
            println!("[ hyper] JSON error: {}", err);
            None
        }
    }
}

// TODO: Replace Option<String> to Result<String, MyError>
#[cfg(any(test, feature="offline"))]
pub async fn get(url: &str) -> Option<String> {
    let response = get_inner(url).await;
    match response {
        Ok(response) => {
            println!("[ hyper] HTTP response: {}", response);
            Some(response)
        },
        Err(err) => {
            println!("[ hyper] HTTP error: {}", err);
            None
        },
    }
}

#[cfg(any(test, feature="offline"))]
async fn get_inner(url: &str) -> Result<String, reqwest::Error> {
    println!("[ hyper] HTTP GET request {}", url);

    let client = reqwest::Client::new();
    let body = client.get(url)
        .header("User-Agent", "github.com/ryanknu/therm_hub")
        .send()
        .await?
        .text()
        .await?;

    Ok(body)
}
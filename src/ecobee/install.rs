use serde::ser::{Serialize, Serializer, SerializeStruct};

// this file covers the ecobee api install process

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResponse {
    ecobee_pin: String,
    code: String,
}

// RK: The `rename_all=camelCase` above will make InstallRepsonse have `ecobeePin` in the JSON
//     but I want `ecobee_pin` in the JSON, and serializers are a lot easier to write than 
//     deserializers, so I'm doing it this way.
impl Serialize for InstallResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 2 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("InstallResponse", 2)?;
        state.serialize_field("ecobee_pin", &self.ecobee_pin)?;
        state.serialize_field("code", &self.code)?;
        state.end()
    }
}

#[cfg(not(any(test, feature="offline")))]
pub async fn install() -> Result<InstallResponse, crate::error::Error> {
    Ok(InstallResponse {
        ecobee_pin: String::from("a263"),
        code: String::from("czTAVXg4thWHhVosrdZPmf8wj0iiKa7A"),
    })
}

#[cfg(any(test, feature="offline"))]
pub async fn install() -> Result<InstallResponse, crate::error::Error> {
    let client_id = std::env::var("ECOBEE_CLIENT_ID").unwrap();
    let url = format!("https://api.ecobee.com/authorize?response_type=ecobeePin&client_id={}&scope=smartRead", client_id);
    let client = reqwest::Client::new();
    let body = client.get(&url)
        .header("User-Agent", "github.com/ryanknu/therm_hub")
        .send()
        .await?
        .text()
        .await?;
    
    crate::parse::<InstallResponse>(&body)
}

use chrono::{NaiveDateTime, Utc};
#[cfg(not(any(test, feature="offline")))]
use crate::http_client::parse;
use crate::schema::ecobee_token;
use diesel::PgConnection;
use diesel::prelude::*;
#[cfg(not(any(test, feature="offline")))]
use std::env;

#[derive(Deserialize, Serialize, Debug)]
pub struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i32,
}

#[derive(Insertable)]
#[table_name="ecobee_token"]
struct TokenInsert {
    access_token: String,
    refresh_token: String,
    expires: NaiveDateTime,
}

#[derive(Clone, Identifiable, Queryable)]
#[table_name="ecobee_token"]
pub struct Token {
    id: i32,
    pub access_token: String,
    pub refresh_token: String,
    expires: NaiveDateTime,
}

impl Token {
    pub fn is_expired(&self) -> bool {
        true
    }
}

pub enum GrantType {
    PIN,
    RefreshToken,
}

impl TokenResponse {
    pub fn to_token(&self) -> Token {
        let expire_seconds: i64 = self.expires_in.into();
        Token {
            id: 0,
            access_token: self.access_token.clone(),
            refresh_token: self.refresh_token.clone(),
            expires: NaiveDateTime::from_timestamp(Utc::now().timestamp() + expire_seconds, 0),
        }
    }
}

pub fn get_token(db: &PgConnection) -> Option<Token> {
    use crate::schema::ecobee_token::dsl;
    match dsl::ecobee_token.select((dsl::id, dsl::access_token, dsl::refresh_token, dsl::expires)).limit(1).load::<Token>(db) {
        Ok(query_result) => {
            match query_result.first() {
                Some(token) => Some(token.clone()),
                None => None,
            }
        },
        Err(_) => None,
    }
}

pub fn save_token(token: &Token, db: &PgConnection) -> Option<Token> {
    use crate::schema::ecobee_token::dsl;
    let query_result = dsl::ecobee_token.select((dsl::id, dsl::access_token, dsl::refresh_token, dsl::expires)).limit(1).load::<Token>(db);

    match query_result {
        Err(err) => {
            println!("[ db   ] Error {:?}", err);
            None
        },
        Ok(query_result) => {
            match query_result.first() {
                None => diesel::insert_into(ecobee_token::table)
                    .values(TokenInsert {
                        access_token: token.access_token.clone(),
                        expires: token.expires,
                        refresh_token: token.refresh_token.clone(),
                    })
                    .get_result(db)
                    .ok(),
                Some(db_token) => diesel::update(db_token)
                    .set((
                        dsl::access_token.eq(token.access_token.clone()), 
                        dsl::expires.eq(token.expires),
                        dsl::refresh_token.eq(token.refresh_token.clone()))
                    )
                    .get_result::<Token>(db)
                    .ok(),
            }
        }
    }
}

#[tokio::main]
pub async fn get_from_remote_blocking(code: &str, grant_type: GrantType) -> Option<TokenResponse> {
    get_from_remote(code, grant_type).await
}

#[allow(unused_variables)]
#[cfg(any(test, feature="offline"))]
pub async fn get_from_remote(code: &str, grant_type: GrantType) -> Option<TokenResponse> {
    Some(TokenResponse {
        access_token: String::from("czTAVXg4thWHhVosrdZPmf8wj0iiKa7A"),
        refresh_token: String::from("czTAVXg4thWHhVosrdZPmf8wj0iiKa7A"),
        expires_in: 3600,
    })
}

#[cfg(not(any(test, feature="offline")))]
pub async fn get_from_remote(code: &str, grant_type: GrantType) -> Option<TokenResponse> {
    let grant_type = match grant_type {
        GrantType::PIN => "ecobeePin",
        GrantType::RefreshToken => "refresh_token",
    };
    let client_id = env::var("ECOBEE_CLIENT_ID").unwrap();
    let url = format!("https://api.ecobee.com/token?grant_type={}&code={}&client_id={}", grant_type, code, client_id);

    match http_request(&url).await {
        Err(err) => {
            println!("[ hyper] HTTP error {}", err);
            None
        },
        Ok(response) => parse::<TokenResponse>(&response),
    }
}

#[cfg(not(any(test, feature="offline")))]
async fn http_request(url: &str) -> Result<String, reqwest::Error> {
    println!("[ hyper] HTTP POST request {}", url);

    let client = reqwest::Client::new();
    let body = client.post(url)
        .header("User-Agent", "github.com/ryanknu/therm_hub")
        .send()
        .await?
        .text()
        .await?;

    Ok(body)
}
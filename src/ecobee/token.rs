#[cfg(not(any(test, feature = "offline")))]
use crate::parse;
use crate::schema::ecobee_token;
use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::PgConnection;
#[cfg(not(any(test, feature = "offline")))]
use std::env;

#[derive(Deserialize, Serialize, Debug)]
pub struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i32,
}

#[derive(Insertable)]
#[table_name = "ecobee_token"]
struct TokenInsert {
    access_token: String,
    refresh_token: String,
    expires: NaiveDateTime,
}

#[derive(Clone, Identifiable, Queryable)]
#[table_name = "ecobee_token"]
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

    let select = dsl::ecobee_token
        .select((dsl::id, dsl::access_token, dsl::refresh_token, dsl::expires))
        .limit(1);

    if cfg!(feature = "queries") {
        println!(
            "{}",
            diesel::debug_query::<diesel::pg::Pg, _>(&select).to_string()
        );
    }

    match select.load::<Token>(db) {
        Ok(query_result) => match query_result.first() {
            Some(token) => Some(token.clone()),
            None => None,
        },
        Err(_) => None,
    }
}

pub fn save_token(token: &Token, db: &PgConnection) -> Option<Token> {
    use crate::schema::ecobee_token::dsl;

    match get_token(db) {
        None => {
            let insert = diesel::insert_into(ecobee_token::table).values(TokenInsert {
                access_token: token.access_token.clone(),
                expires: token.expires,
                refresh_token: token.refresh_token.clone(),
            });

            if cfg!(feature = "queries") {
                println!(
                    "{}",
                    diesel::debug_query::<diesel::pg::Pg, _>(&insert).to_string()
                );
            }

            insert.get_result(db).ok()
        }
        Some(db_token) => {
            let update = diesel::update(&db_token).set((
                dsl::access_token.eq(token.access_token.clone()),
                dsl::expires.eq(token.expires),
                dsl::refresh_token.eq(token.refresh_token.clone()),
            ));

            if cfg!(feature = "queries") {
                println!(
                    "{}",
                    diesel::debug_query::<diesel::pg::Pg, _>(&update).to_string()
                );
            }
            update.get_result::<Token>(db).ok()
        }
    }
}

#[tokio::main]
pub async fn get_from_remote_blocking(
    code: &str,
    grant_type: GrantType,
) -> Result<TokenResponse, crate::error::Error> {
    get_from_remote(code, grant_type).await
}

#[allow(unused_variables)]
#[cfg(any(test, feature = "offline"))]
pub async fn get_from_remote(
    code: &str,
    grant_type: GrantType,
) -> Result<TokenResponse, crate::error::Error> {
    Ok(TokenResponse {
        access_token: String::from("czTAVXg4thWHhVosrdZPmf8wj0iiKa7A"),
        refresh_token: String::from("czTAVXg4thWHhVosrdZPmf8wj0iiKa7A"),
        expires_in: 3600,
    })
}

#[cfg(not(any(test, feature = "offline")))]
pub async fn get_from_remote(
    code: &str,
    grant_type: GrantType,
) -> Result<TokenResponse, crate::error::Error> {
    let grant_type = match grant_type {
        GrantType::PIN => "ecobeePin",
        GrantType::RefreshToken => "refresh_token",
    };
    let client_id = env::var("ECOBEE_CLIENT_ID").unwrap();
    let url = format!(
        "https://api.ecobee.com/token?grant_type={}&code={}&client_id={}",
        grant_type, code, client_id
    );

    let client = reqwest::Client::new();
    let body = client
        .post(&url)
        .header("User-Agent", "github.com/ryanknu/therm_hub")
        .send()
        .await?
        .text()
        .await?;

    parse::<TokenResponse>(&body)
}

/// # Current Token
/// Retrieves a current token from the DB or refreshes and saves the token from the remote API.
pub fn current_token(db: &PgConnection) -> Option<Token> {
    match get_token(db) {
        None => None,
        Some(token) => match token.is_expired() {
            false => Some(token.clone()),
            true => match get_from_remote_blocking(&token.refresh_token, GrantType::RefreshToken) {
                Err(_) => None,
                Ok(response) => {
                    save_token(&response.to_token(), db);
                    Some(response.to_token())
                }
            },
        },
    }
}

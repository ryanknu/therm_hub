use diesel::PgConnection;

// Re-export everything used in other crates, so they do not need to know the module structure.
pub use install::install;
pub use reading::Reading;
pub use reading::read;
pub use token::Token;
pub use token::get_from_remote as get_token;
pub use token::save_token;
pub use token::GrantType::PIN as GRANT_PIN;

mod install;
mod reading;
mod token;

pub fn current_token(db: &PgConnection) -> Option<Token> {
    match token::get_token(db) {
        None => None,
        Some(token) => {
            match token.is_expired() {
                false => Some(token.clone()),
                true => {
                    let result = token::get_from_remote_blocking(&token.refresh_token, token::GrantType::RefreshToken);
                    match result {
                        Err(_) => None,
                        Ok(response) => {
                            token::save_token(&response.to_token(), db);
                            Some(response.to_token())
                        }
                    }
                },
            }
        },
    }
}

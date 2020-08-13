use diesel::PgConnection;
pub use reading::Reading;
pub use reading::read;
pub use token::Token;

pub mod install;
pub mod reading;
pub mod token;

pub fn current_token(db: &PgConnection) -> Option<Token> {
    match token::get_token(db) {
        None => None,
        Some(token) => {
            match token.is_expired() {
                false => Some(token.clone()),
                true => {
                    let result = token::get_from_remote_blocking(&token.refresh_token, token::GrantType::RefreshToken);
                    match result {
                        None => None,
                        Some(response) => {
                            token::save_token(&response.to_token(), db);
                            Some(response.to_token())
                        }
                    }
                },
            }
        },
    }
}

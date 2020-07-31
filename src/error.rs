use std::error::Error as StdError;
use std::fmt;

// TODO: THIS WHOLE FILE

#[derive(Debug)]
pub struct Error {
    message: String,
}
pub type Result<T> = ::std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref url) = self.url {
            try!(fmt::Display::fmt(url, f));
            try!(f.write_str(": "));
        }
        match self.kind {
            Kind::Http(ref e) => fmt::Display::fmt(e, f),
            Kind::Url(ref e) => fmt::Display::fmt(e, f),
            Kind::Tls(ref e) => fmt::Display::fmt(e, f),
            Kind::Io(ref e) => fmt::Display::fmt(e, f),
            Kind::UrlEncoded(ref e) => fmt::Display::fmt(e, f),
            Kind::Json(ref e) => fmt::Display::fmt(e, f),
            Kind::TooManyRedirects => f.write_str("Too many redirects"),
            Kind::RedirectLoop => f.write_str("Infinite redirect loop"),
            Kind::ClientError(ref code) => {
                f.write_str("Client Error: ")?;
                fmt::Display::fmt(code, f)
            }
            Kind::ServerError(ref code) => {
                f.write_str("Server Error: ")?;
                fmt::Display::fmt(code, f)
            }
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match self.kind {
            Kind::Http(ref e) => e.description(),
            Kind::Url(ref e) => e.description(),
            Kind::Tls(ref e) => e.description(),
            Kind::Io(ref e) => e.description(),
            Kind::UrlEncoded(ref e) => e.description(),
            Kind::Json(ref e) => e.description(),
            Kind::TooManyRedirects => "Too many redirects",
            Kind::RedirectLoop => "Infinite redirect loop",
            Kind::ClientError(_) => "Client Error",
            Kind::ServerError(_) => "Server Error",
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match self.kind {
            Kind::Http(ref e) => e.cause(),
            Kind::Url(ref e) => e.cause(),
            Kind::Tls(ref e) => e.cause(),
            Kind::Io(ref e) => e.cause(),
            Kind::UrlEncoded(ref e) => e.cause(),
            Kind::Json(ref e) => e.cause(),
            Kind::TooManyRedirects |
            Kind::RedirectLoop |
            Kind::ClientError(_) |
            Kind::ServerError(_) => None,
        }
    }
}

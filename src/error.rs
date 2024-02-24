// Needed imports
use crate::Reply;
use crate::routes::{
  html,
  set_status,
};
use hyper::{
//  Request,
//  Response,
  StatusCode,
};
use askama::Template;
use serde::Serialize;
// Public errors to wrap
use hyper::header::ToStrError as UnreadableHeaderError;
use serde_json::Error as JsonError;
use serde_urlencoded::de::Error as UrlEncodingError;
use std::num::ParseIntError;
// Private errors to wrap
use hyper::Error as ConnectionError;
use hyper::header::InvalidHeaderValue;
use sqlx::error::Error as SqlxError;
use openidconnect::ClaimsVerificationError as OIDCClaimsVerificationError;
use askama::Error as RenderingError;

type OIDCRequestError = openidconnect::RequestTokenError<
  openidconnect::reqwest::Error<reqwest::Error>,
  openidconnect::StandardErrorResponse<openidconnect::core::CoreErrorResponseType>
>;

use crate::traits::{
  Request,
  Response,
};

// Templates for error pages
//
// They use HTMX tags to insert the message into a page for HTMX requests, but
// are also still HTML pages.
#[derive(Template)]
#[template(path = "global_error.html")]
struct GlobalUIError{
  message: String,
}
#[derive(Template)]
#[template(path = "input_error.html")]
struct InputUIError{
  message: String,
}

// Error representation for internal errors
// Prints to stderr and returns a http 500 internal error
#[derive(Debug)]
pub enum InternalError {
  Connection(ConnectionError),
  InvalidHeader(InvalidHeaderValue),
  Db(SqlxError),
  OIDCRequestError(OIDCRequestError),
  TamperedOIDCLogin(OIDCClaimsVerificationError),
  RenderingError(RenderingError),
}
impl std::fmt::Display for InternalError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "{:?}", self)
  }
}
impl Reply for InternalError {
  fn into_response(self) -> Response {
    eprintln!("{}", &self);
    // By using a constant instance of ClientError formatting is consistent
    ClientError::InternalError.into_response()
  }
}
impl From<ConnectionError> for InternalError {
  fn from(e: ConnectionError) -> Self {
    InternalError::Connection(e)
  }
}
impl From<SqlxError> for InternalError {
  fn from(e: SqlxError) -> Self {
    InternalError::Db(e)
  }
}

#[derive(Debug, Serialize)]
pub enum ClientError {
  InternalError,

  // Routing errors
  PathNotFound(String),
  MethodNotFound{method: String, path: String},
  Forbidden,

  // Parsing errors
  PathDataBeforeRoot(String),
  UnreadableHeader(String),
  UnparseableCookie(String),
  DuplicateCookies{name: String, value: String, old_value: String},
  InvalidContentLength(String),
  InvalidContentType(String),
  InvalidJson(String),
  InvalidUrlEncoding(String),
  InvalidIndexPath(String),

  // Request processing errors
  AlreadyExists(String), // For example uniqueness error on name column
  InvalidData(String), // For example no name given

  // Non-parsing user-caused errors (but probably not intentional)
  UnknownOIDCProcess, // Post-login OIDC handler did not find the OIDC login in DB
  OIDCGaveNoToken, // Unlikely, would probably be error in OIDC provider
  OIDCGaveNoEmail, // Probably won't happen

  UserNotFound(String), // Suggests contacting the site admin to register an account
}
impl Reply for ClientError {
  fn into_response(self) -> Response {
    // Match on the error, to give a proper text description and HTTP status
    let (status, body) = match self {
      Self::InternalError => (
        StatusCode::INTERNAL_SERVER_ERROR,
        GlobalUIError{
          message: "Internal server error. Please try again in a few minutes.".into(),
        }.render(),
      ),

      Self::PathNotFound(p) => (
        StatusCode::NOT_FOUND,
        GlobalUIError{
          message: format!("Path {p} not found."),
        }.render(),
      ),
      Self::MethodNotFound{method, path} => (
        StatusCode::METHOD_NOT_ALLOWED,
        GlobalUIError{
          message: format!("Method {method} not valid for path {path}."),
        }.render(),
      ),
      Self::Forbidden => (
        StatusCode::FORBIDDEN,
        GlobalUIError{
          message: "Operation forbidden!".into(),
        }.render(),
      ),

      Self::AlreadyExists(message) => (
        StatusCode::CONFLICT,
        InputUIError{
          message,
        }.render(),
      ),
      Self::UserNotFound(email) => (
        StatusCode::FORBIDDEN,
        GlobalUIError{
          message: format!("No account exists for gmail {email}. Contact admins to create one."),
        }.render(),
      ),

      // The rest are weird formatting, so report that the user's browser is weird
      x => (
        StatusCode::BAD_REQUEST,
        GlobalUIError{
          message: format!(
            "Bad request. Your browser is misbehaving. Error: {}",
            serde_json::to_string(&x).unwrap(), // Only errors if struct can't be represented as json
          ),
        }.render(),
      ),
    };

    // Construct a response from that
    match body {
      Ok(b) => {
        let ret = html(b);
        set_status(ret, status)
      },
      Err(e) => {
        eprintln!("{}", e);
        let ret = html(
r#"<div id="global-error" hx-swap-oob=true>
  Server is on proverbial fire. Kindly give us a while to recover.
<\div>"#
        );
        set_status(ret, StatusCode::INTERNAL_SERVER_ERROR)
      },
    }.unwrap() // set_status returns option for ergonomics but is infallible
  }
}

// Enum over both internal and client error
// allows us treating both consistently
#[derive(Debug)]
pub enum Error {
  InternalError(InternalError),
  ClientError(ClientError),
}
// Utility constructors
// We use .into() to convert ClientError into Error
impl Error {
  pub fn path_data_before_root(data: String) -> Self {
    ClientError::PathDataBeforeRoot(data).into()
  }
  pub fn path_not_found(req: &Request) -> Self {
    ClientError::PathNotFound(req.uri().path().to_owned()).into()
  }
  pub fn method_not_found(req: &Request) -> Self {
    ClientError::MethodNotFound{
      method: req.method().to_string(),
      path: req.uri().path().to_string(),
    }.into()
  }
  pub fn forbidden() -> Self {
    ClientError::Forbidden.into()
  }
  // Where multiple parsing errors give the same error type
  // we need to use a function for one of the cases
  pub fn unreadable_header(e: UnreadableHeaderError, header: &str) -> Self {
    ClientError::UnreadableHeader(format!(
      "Error reading header {}: {}",
      header, e,
    )).into()
  }
  pub fn unparseable_cookie(cookie_data: &str) -> Self {
    ClientError::UnparseableCookie(cookie_data.into()).into()
  }
  pub fn duplicate_cookies(name: &str, value: &str, old_value: &str) -> Self {
    ClientError::DuplicateCookies{
      name: name.into(),
      value: value.into(),
      old_value: old_value.into(),
    }.into()
  }

  pub fn content_length_missing() -> Self {
    ClientError::InvalidContentLength(
      "No content length given".to_string()
    ).into()
  }
  pub fn content_length_not_int(err: ParseIntError) -> Self {
    ClientError::InvalidContentLength(format!(
      "Invalid unsigned int: {}",
      err,
    )).into()
  }
  pub fn content_length_too_large(parsed: usize, max: usize) -> Self {
    ClientError::InvalidContentLength(format!(
      "Too large. Maximum allowed is {}, received {}",
      max, parsed,
    )).into()
  }
  pub fn content_length_mismatch(given: usize, promised: usize) -> Self {
    let at_least = if given > promised {" at least"} else {""};
    ClientError::InvalidContentLength(format!(
      "Mismatch. Header is {}, received {} {}",
      promised, at_least, given,
    )).into()
  }
  pub fn invalid_content_type(parsed: &str, expected: &str) -> Self {
    ClientError::InvalidContentType(format!(
      "Expected {}, received {}",
      parsed, expected
    )).into()
  }
}

// Implementing Reply on this error type enables rust to convert any error into
// the correct response to the client (with a print to stderr for internal).
impl Reply for Error {
  fn into_response(self) -> Response {
    match self {
      Self::InternalError(e) => e.into_response(),
      Self::ClientError(e) => e.into_response(),
    }
  }
}

// Implementing these allows '?' and .into() to convert them all into our Error
impl From<InternalError> for Error {
  fn from(e: InternalError) -> Self {
    Self::InternalError(e)
  }
}
impl From<ClientError> for Error {
  fn from(e: ClientError) -> Self {
    Self::ClientError(e)
  }
}

impl From<JsonError> for Error {
  fn from(e: JsonError) -> Self {
    ClientError::InvalidJson(format!("{}", e)).into()
  }
}
impl From<UrlEncodingError> for Error {
  fn from(e: UrlEncodingError) -> Self {
    ClientError::InvalidUrlEncoding(format!("{}", e)).into()
  }
}
impl From<ParseIntError> for Error {
  fn from(e: ParseIntError) -> Self {
    ClientError::InvalidIndexPath(format!("{}", e)).into()
  }
}
impl From<SqlxError> for Error {
  fn from(e: SqlxError) -> Self {
    InternalError::Db(e).into()
  }
}
impl From<ConnectionError> for Error {
  fn from(e: ConnectionError) -> Self {
    InternalError::Connection(e).into()
  }
}
// most likely created by an invalid redirect
impl From<InvalidHeaderValue> for Error {
  fn from(e: InvalidHeaderValue) -> Self {
    InternalError::InvalidHeader(e).into()
  }
}
impl From<OIDCClaimsVerificationError> for Error {
  fn from(e: OIDCClaimsVerificationError) -> Self {
    InternalError::TamperedOIDCLogin(e).into()
  }
}
impl From<OIDCRequestError> for Error {
  fn from(e: OIDCRequestError) -> Self {
    InternalError::OIDCRequestError(e).into()
  }
}
impl From<RenderingError> for Error {
  fn from(e: RenderingError) -> Self {
    InternalError::RenderingError(e).into()
  }
}

use super::*;

//
// Request deconstructors
//

// Get a specific header as string reference if exists, else None
pub fn get_header<'a>(
  req: &'a Request,
  header_name: &str,
) -> Result<Option<&'a str>, Error> {
  Ok(match req.headers().get(header_name) {
    Some(val) => Some(
      val
        .to_str()
        .map_err(|e| Error::unreadable_header(e, header_name))?,
    ),
    None => None,
  })
}
// Verify content len, should be done before calling get_body or any of the
// other functions below that accept the rest of a multipart request
pub fn validate_get_content_len<'a>(
  req: &'a Request,
  max_len: usize,
) -> Result<usize, Error> {
  let header = get_header(&req, "Content-Length")?;
  if let Some(x) = header {
    let length = x.parse::<usize>().map_err(Error::content_length_not_int)?;
    if length <= max_len {
      Ok(length)
    } else {
      Err(Error::content_length_too_large(length, max_len))
    }
  } else {
    Err(Error::content_length_missing())
  }
}
// Try to parse the uri query part as urlencoded into object of type T
// T to parse into is set to what you save the return value into
pub fn parse_query<T: DeserializeOwned>(
  req: &Request,
) -> Result<T, Error> {
  let query_str = req.uri().query().unwrap_or("");
  let filter: T = serde_urlencoded::from_str(query_str)?;
  Ok(filter)
}
pub fn parse_cookies<'a>(
  req: &'a Request,
) -> Result<std::collections::HashMap<&'a str, &'a str>, Error> {
  match req.headers().get(hyper::header::COOKIE) {
    Some(h) => {
      let raw = h.to_str().map_err(|e| Error::unreadable_header(e, "cookie"))?;
      let mut parsed = std::collections::HashMap::new();
      for cookie in raw.split(&[';',' ']) {
        if cookie != "" {
          let (name, value) = cookie.split_once('=')
            .ok_or(Error::unparseable_cookie(raw))?
          ;
          match parsed.insert(name, value) {
            Some(old_value) => return Err(Error::duplicate_cookies(name, value, old_value)),
            None => {}
          };
        }
      }
      Ok(parsed)
    },
    // If no cookies they cannot be authenticated
    None => { Ok([].into()) },
  }
}
// Wait for and save packets from client until transmission ends _OR_
// more bytes that Content-Length have been received (error).
// Uses validate_get_content_len to verify Content-Length < max_len
// (
//  More performant than stream processing because Serde performs better
//  on continuous memory, such as a list of bytes.
// )
pub async fn get_body(
  req: &mut Request,
  max_len: usize,
) -> Result<Vec<u8>, Error> {
  use hyper::body::Body; // Needed for size_hint operations
  use http_body_util::BodyExt; // Provides the .frame() future on body

  // First we validate and set up
  let expected_len = validate_get_content_len(req, max_len)?;
  let mut bytes = Vec::with_capacity(expected_len);
  let body = req.body_mut();
  futures::pin_mut!(body);

  // Then we loop until we either overshoot Content-Len and error or
  // run out of data and return what we got
  while let Some(result) = body.frame().await {
    // It could be an error or a trailer frame, so check that it is valid data
    // Should be an &hyper::body::Bytes after this
    let data = match result?.into_data() {
      Ok(data) => data,
      // If Err returned this is a trailer frame, which both signifies that
      // there won't be more data and we don't care about.
      Err(_original_frame) => break,
    };
    // Check against overrunning
    if bytes.len() + data.len() > expected_len {
      // If we overrun try to estimate length of received request
      let estimate = bytes.len() + data.len() + body.size_hint().lower() as usize;
      return Err(Error::content_length_mismatch(estimate, expected_len));
    }
    // As hyper::body::Bytes has Deref to [u8] we simply copy
    bytes.extend_from_slice(&data);
  }

  // Finally check against undershooting
  if bytes.len() < expected_len {
    Err(Error::content_length_mismatch(bytes.len(), expected_len))
  } else {
    Ok(bytes)
  }
}
// Try to parse the body of the request as form submission into object of type T
// T to parse into is set to what you save the return value into
pub async fn parse_body_urlencoded<T: DeserializeOwned>(
  req: &mut Request,
  max_len: usize,
) -> Result<T, Error> {
  // Verify content type
  let content_type = get_header(req, "Content-Type")?.unwrap_or("");
  if "application/x-www-form-urlencoded" != content_type {
    return Err(Error::invalid_content_type(
      "application/x-www-form-urlencoded",
      content_type,
    ));
  }
  // Get body
  let bytes = get_body(req, max_len).await?;
  // Try to parse
  let data: T = serde_urlencoded::from_bytes(&bytes)?;
  Ok(data)
}

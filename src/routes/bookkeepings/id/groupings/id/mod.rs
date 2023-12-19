use super::*;

// Only data (no calculations), since this will be fetched very often
#[derive(Debug)]
pub struct Grouping {
  id: i64,
  name: String,
}

async fn index(
) -> Result<Response, Error> {
  html("todo")
}

pub async fn route(
  state: &'static State,
  req: Request,
  mut path_vec: Vec<String>,
  session: SessionData,
  bookkeeping: Bookkeeping,
  grouping_id: i64,
) -> Result<Response, Error> {
  // Get the grouping, both to verify existence and that it belongs to this
  // bookkeeping (and also since routes are likely to want the name)
  let grouping = sqlx::query_as!(Grouping,
    "
SELECT Groupings.id, Groupings.name
  FROM Groupings
WHERE Groupings.bookkeeping_id = $1 AND Groupings.id = $2
    ",
    bookkeeping.id,
    grouping_id,
  )
    .fetch_optional(&state.db)
    .await?
    .ok_or(Error::path_not_found(&req))?
  ;
  match path_vec.pop().as_deref() {
    None => permanent_redirect(&format!("{}/", req.uri().path())),
    Some("") => {
      verify_path_end(&path_vec, &req)?;
      match req.method() {
        &Method::GET => index().await,
        _ => Err(Error::method_not_found(&req)),
      }
    },
    _ => Err(Error::path_not_found(&req)),
  }
}

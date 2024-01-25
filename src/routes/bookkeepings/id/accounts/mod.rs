use super::*;

#[derive(Debug, Deserialize)]
struct NewAccount {
  name: String,
  r#type: String,
}
#[derive(Debug, Template)]
#[template(path = "bookkeepings/id/account-entry.part.html")]
struct IndexPost {
  a: AccountSummary,
}
async fn index_post(
  state: &'static State,
  mut req: Request,
  session: SessionData,
  bookkeeping_id: i64,
) -> Result<Response, Error> {
  // Parse out the new account
  let new_account: NewAccount = parse_body_urlencoded(
    &mut req,
    state.max_content_len,
  ).await?;
  // Insert into database
  let created = sqlx::query_as!(AccountSummary,
    "
INSERT INTO Accounts(name, type, bookkeeping_id) VALUES($1, $2, $3)
RETURNING id, name, type, 0 AS \"balance!\"
    ",
    new_account.name,
    new_account.r#type,
    bookkeeping_id,
  )
    .fetch_one(&state.db)
    .await
    .map_err(|e| -> Error { match e {
      sqlx::Error::Database(ref dbe) if dbe.is_unique_violation() => {
        ClientError::AlreadyExists(format!(
          "An account by name {} already exists in this bookkeeping.",
          new_account.name,
        )).into()
      },
      e => e.into(),
    }})
    ?
  ;
  // Redirect to parent with query parameter of created account's id
  html(IndexPost{
    a: created
  }.render()?)
}
pub async fn route(
  state: &'static State,
  req: Request,
  mut path_vec: Vec<String>,
  session: SessionData,
  bookkeeping: Bookkeeping,
) -> Result<Response, Error> {
  match path_vec.pop().as_deref() {
    None => permanent_redirect(&format!("{}/", req.uri().path())),
    Some("") => {
      verify_path_end(&path_vec, &req)?;
      match req.method() {
        &Method::POST => index_post(state, req, session, bookkeeping.id).await,
        _ => Err(Error::method_not_found(&req)),
      }
    },
    _ => Err(Error::path_not_found(&req)),
  }
}

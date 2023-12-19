use super::*;

#[derive(Debug)]
struct AccountSummary {
  id: i64,
  name: String,
  r#type: String,
  balance: Decimal,
}
#[derive(Debug, Template)]
#[template(path = "bookkeepings/id/accounts/index.html")]
struct Index {
  accounts: Vec<AccountSummary>,
}

async fn index(
  state: &'static State,
  req: Request,
  session: SessionData,
  bookkeeping_id: i64,
) -> Result<Response, Error> {
  let a = sqlx::query_as!(AccountSummary,
    "
SELECT Accounts.id, Accounts.name, Accounts.type, COALESCE(SUM(AccountChanges.amount), 0) AS \"balance!\"
  FROM Accounts
  LEFT JOIN AccountChanges ON AccountChanges.account_id = Accounts.id
WHERE Accounts.bookkeeping_id = $1
GROUP BY Accounts.id, Accounts.name
    ",
    bookkeeping_id,
  )
    .fetch_all(&state.db)
    .await?
  ;
  println!("{a:?}");

  html(Index{
    accounts: a,
  }.render()?)
}
#[derive(Debug, Deserialize)]
struct NewAccount {
  name: String,
  r#type: String,
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
  let created = sqlx::query!(
    "INSERT INTO Accounts(name, type, bookkeeping_id) VALUES($1, $2, $3) RETURNING id",
    new_account.name,
    new_account.r#type,
    bookkeeping_id,
  )
    .fetch_one(&state.db)
    .await
    // TODO Convert
    //   if is_unique_violation mark duplicate account
    //   if is_foreign_key_violation mark bad request
    ?
    .id
  ;
  // Redirect to parent with query parameter of created account's id
  see_other(&format!("../?new_account={created}"))
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
//        &Method::GET => index(state, req, session, bookkeeping_id).await,
        &Method::POST => index_post(state, req, session, bookkeeping.id).await,
        _ => Err(Error::method_not_found(&req)),
      }
    },
    _ => Err(Error::path_not_found(&req)),
  }
}

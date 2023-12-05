use super::*;

#[derive(Debug)]
struct Bookkeeping {
  name: String,
  owner: String,
}
#[derive(Debug)]
struct AccountSummary {
  id: i64,
  name: String,
  balance: Decimal,
}
#[derive(Debug)]
struct GroupingSummary {
  id: i64,
  name: String,
  movement: Decimal,
}
#[derive(Debug, Template)]
#[template(path = "bookkeepings/id/index.html")]
struct Index {
  name: String,
  owner: String,
  accounts: Vec<AccountSummary>,
  groupings: Vec<GroupingSummary>,
}

async fn index(
  state: &'static State,
  req: Request,
  session: SessionData,
  bookkeeping_id: i64,
) -> Result<Response, Error> {
  // Get the bookkeeping itself
  let b = sqlx::query_as!(Bookkeeping,
    "
SELECT Bookkeepings.name, Users.email AS owner
  FROM Bookkeepings
  JOIN Users ON Users.id = Bookkeepings.owner_id
WHERE Bookkeepings.id = $1
    ",
    bookkeeping_id
  )
    .fetch_optional(&state.db)
    .await?
    .ok_or(Error::path_not_found(&req))?
  ;
  let a = sqlx::query_as!(AccountSummary,
    "
SELECT Accounts.id, Accounts.name, SUM(AccountChanges.amount) AS \"balance!\"
  FROM Accounts
  JOIN AccountChanges ON AccountChanges.account_id = Accounts.id
WHERE Accounts.bookkeeping_id = $1
GROUP BY Accounts.id, Accounts.name
    ",
    bookkeeping_id,
  )
    .fetch_all(&state.db)
    .await?
  ;
  let g = sqlx::query_as!(GroupingSummary,
    "
SELECT Groupings.id, Groupings.name, SUM(CASE
    WHEN AccountChanges.amount < 0 THEN -AccountChanges.amount
    ELSE AccountChanges.amount
  END) AS \"movement!\"
  FROM Groupings
  JOIN Transactions ON Transactions.grouping_id = Groupings.id
  JOIN AccountChanges ON AccountChanges.transaction_id = Transactions.id
WHERE Groupings.bookkeeping_id = $1
GROUP BY Groupings.id
    ",
    bookkeeping_id,
  )
    .fetch_all(&state.db)
    .await?
  ;

  html(Index{
    name: b.name,
    owner: b.owner,
    accounts: a,
    groupings: g,
  }.render()?)
}
pub async fn route(
  state: &'static State,
  req: Request,
  mut path_vec: Vec<String>,
  session: SessionData,
  bookkeeping_id: i64,
) -> Result<Response, Error> {
  match path_vec.pop().as_deref() {
    None => permanent_redirect(&format!("{}/", req.uri().path())),
    Some("") => {
      verify_method_path_end(&path_vec, &req, &Method::GET)?;
      index(state, req, session, bookkeeping_id).await
    },
    _ => Err(Error::path_not_found(&req)),
  }
}

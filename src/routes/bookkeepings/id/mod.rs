use super::*;

mod accounts;
mod groupings;
mod imported_account_changes;

#[derive(Debug)]
pub struct Bookkeeping {
  id: i64,
  name: String,
  owner: String,
}
#[derive(Debug)]
struct AccountSummary {
  id: i64,
  name: String,
  r#type: String,
  balance: Decimal,
}
#[derive(Debug)]
struct AccountType {
  name: String,
}
#[derive(Debug)]
struct GroupingSummary {
  id: i64,
  name: String,
  movement: Decimal,
}
#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Created {
  new_account: Option<i64>,
  new_grouping: Option<i64>,
}
impl Created {
  fn equals_account(&self, id: &i64) -> bool {
    self.new_account == Some(*id)
  }
  fn equals_grouping(&self, id: &i64) -> bool {
    self.new_grouping == Some(*id)
  }
}
#[derive(Debug, Template)]
#[template(path = "bookkeepings/id/index.html")]
struct Index {
  name: String,
  owner: String,
  accounts: Vec<AccountSummary>,
  groupings: Vec<GroupingSummary>,
  account_types: Vec<AccountType>,
  created: Created,
}

async fn index(
  state: &'static State,
  req: Request,
  session: SessionData,
  bookkeeping: Bookkeeping,
  query: Created,
) -> Result<Response, Error> {
  let a = sqlx::query_as!(AccountSummary,
    "
SELECT Accounts.id, Accounts.name, Accounts.type, COALESCE(SUM(AccountChanges.amount), 0) AS \"balance!\"
  FROM Accounts
  LEFT JOIN AccountChanges ON AccountChanges.account_id = Accounts.id
WHERE Accounts.bookkeeping_id = $1
GROUP BY Accounts.id, Accounts.name, Accounts.type
ORDER BY Accounts.type, Accounts.name
    ",
    bookkeeping.id,
  )
    .fetch_all(&state.db)
    .await?
  ;
  let g = sqlx::query_as!(GroupingSummary,
    "
SELECT Groupings.id, Groupings.name, COALESCE(SUM(CASE
    WHEN AccountChanges.amount > 0 THEN AccountChanges.amount
    ELSE 0
  END),0) AS \"movement!\"
  FROM Groupings
  LEFT JOIN Transactions ON Transactions.grouping_id = Groupings.id
  LEFT JOIN AccountChanges ON AccountChanges.transaction_id = Transactions.id
WHERE Groupings.bookkeeping_id = $1
GROUP BY Groupings.id
    ",
    bookkeeping.id,
  )
    .fetch_all(&state.db)
    .await?
  ;
  let t = sqlx::query_as!(AccountType,
    "SELECT name FROM AccountTypes",
  )
    .fetch_all(&state.db)
    .await?
  ;

  html(Index{
    name: bookkeeping.name,
    owner: bookkeeping.owner,
    accounts: a,
    groupings: g,
    account_types: t,
    created: query,
  }.render()?)
}
pub async fn route(
  state: &'static State,
  req: Request,
  mut path_vec: Vec<String>,
  session: SessionData,
  bookkeeping_id: i64,
) -> Result<Response, Error> {
  // Get the bookkeeping, both to verify permissions to/existence of the
  // bookkeeping and since most routes want to at least print the name
  let bookkeeping = sqlx::query_as!(Bookkeeping,
    "
SELECT Bookkeepings.id, Bookkeepings.name, Users.email AS owner
  FROM Bookkeepings
  LEFT JOIN UsersBookkeepingsAccess ON Bookkeepings.id = bookkeeping_id
  JOIN Users ON Users.id = Bookkeepings.owner_id
WHERE (Bookkeepings.owner_id = $1 OR UsersBookkeepingsAccess.user_id = $1)
  AND Bookkeepings.id = $2
    ",
    session.user_id,
    bookkeeping_id
  )
    .fetch_optional(&state.db)
    .await?
    .ok_or(Error::path_not_found(&req))?
  ;
  match path_vec.pop().as_deref() {
    None => permanent_redirect(&format!("{}/", req.uri().path())),
    Some("") => {
      verify_method_path_end(&path_vec, &req, &Method::GET)?;
      let created: Created = parse_query(&req)?;
      index(state, req, session, bookkeeping, created).await
    },
    Some("accounts") => accounts::route(state, req, path_vec, session, bookkeeping).await,
    Some("groupings") => groupings::route(state, req, path_vec, session, bookkeeping).await,
    Some("imported_account_changes") => imported_account_changes::route(state, req, path_vec, session, bookkeeping).await,
    _ => Err(Error::path_not_found(&req)),
  }
}

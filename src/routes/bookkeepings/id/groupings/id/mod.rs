use super::*;

mod transactions;

// Only data (no calculations), since this will be fetched very often
#[derive(Debug)]
pub struct Grouping {
  id: i64,
  name: String,
}

#[derive(Debug)]
pub struct TransactionSummary {
  id: i64,
  name: String,
  date: Date,
  sum: Decimal,
}
#[derive(Debug, Template)]
#[template(path = "bookkeepings/id/groupings/id/index.html")]
struct Index {
  name: String,
  bookkeeping_name: String,
  accounts: Vec<AccountSummary>,
  transactions: Vec<TransactionSummary>,
  accounts_by_type: std::collections::HashMap::<String, Vec<AccountSummary>>,
}
// Give a summary over the grouping, just like for bookkeepings above
async fn index(
  state: &'static State,
  session: SessionData,
  bookkeeping: Bookkeeping,
  grouping: Grouping,
) -> Result<Response, Error> {
  let a = sqlx::query_as!(AccountSummary,
    "
SELECT id AS \"id!\", name AS \"name!\", type AS \"type!\", COALESCE(SUM(amount), 0) AS \"balance!\"
FROM (
  SELECT Accounts.id, Accounts.name, Accounts.type, AccountChanges.amount
    FROM Accounts
    LEFT JOIN AccountChanges ON AccountChanges.account_id = Accounts.id
    LEFT JOIN Transactions ON Transactions.id = AccountChanges.transaction_id
  WHERE Transactions.grouping_id = $2
  UNION ALL
  SELECT Accounts.id, Accounts.name, Accounts.type, 0 AS amount
    FROM Accounts
  WHERE Accounts.bookkeeping_id = $1
)
GROUP BY id, name, type
ORDER BY type, name
    ",
    bookkeeping.id,
    grouping.id,
  )
    .fetch_all(&state.db)
    .await?
  ;
  let t = sqlx::query_as!(TransactionSummary,
    "
SELECT Transactions.id, Transactions.name, Transactions.day AS date,
    COALESCE(SUM(AccountChanges.amount), 0) AS \"sum!\"
  FROM Transactions
  LEFT JOIN AccountChanges ON AccountChanges.transaction_id = Transactions.id
WHERE Transactions.grouping_id = $1
GROUP BY Transactions.id, Transactions.name, Transactions.day
ORDER BY Transactions.day
    ",
    grouping.id,
  )
    .fetch_all(&state.db)
    .await?
  ;
  // Group copies of accounts by account type
  let mut accounts_by_type = std::collections::HashMap::<String, Vec<AccountSummary>>::new();
  for account in &a {
    match accounts_by_type.get_mut(&account.r#type) {
      Some(x) => x.push(account.clone()),
      None => { accounts_by_type.insert(account.r#type.clone(), vec![account.clone()]); },
    }
  }
  html(Index{
    name: grouping.name,
    bookkeeping_name: bookkeeping.name,
    accounts: a,
    transactions: t,
    accounts_by_type,
  }.render()?)
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
        &Method::GET => {
          index(state, session, bookkeeping, grouping).await
        },
        _ => Err(Error::method_not_found(&req)),
      }
    },
    Some("transactions") => transactions::route(
      state,
      req,
      path_vec,
      session,
      bookkeeping,
      grouping,
    ).await,
    _ => Err(Error::path_not_found(&req)),
  }
}

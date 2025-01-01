use super::*;

// Duplicate declaration to groupings/id/transactions/id/mod.rs
// Kept since the usage may differ in the future
#[derive(Debug)]
struct Account{
  id: i64,
  name: String,
  t: String,
}
#[derive(Debug)]
struct ImportedAccountChange {
  id: i64,
  account_name: String,
  date: Date,
  amount: Decimal,
  other_data: sqlx::types::JsonValue,
}
#[derive(Debug, Template)]
#[template(path = "bookkeepings/id/imported_account_changes/index.html")]
struct Index {
  bookkeeping_name: String,
  imported_account_changes: Vec<ImportedAccountChange>,
  accounts_by_type: std::collections::HashMap<String, Vec<Account>>,
}
async fn index(
  state: &'static State,
  req: Request,
  session: SessionData,
  bookkeeping: Bookkeeping,
) -> Result<Response, Error> {
  // Get all the imported account changes valid for this bookkeeping
  let imported_account_changes = sqlx::query_as!(ImportedAccountChange,
    "
SELECT ImportedAccountChanges.id, Accounts.name AS account_name,
    ImportedAccountChanges.day as date, ImportedAccountChanges.amount,
    ImportedAccountCHanges.other_data
  FROM ImportedAccountChanges
  INNER JOIN Accounts ON ImportedAccountChanges.account_id = Accounts.id
WHERE Accounts.bookkeeping_id = $1
ORDER BY ImportedAccountChanges.day
    ",
    bookkeeping.id
  )
    .fetch_all(&state.db)
    .await?
  ;
  // We need all the accounts (by type) for the form creating account changes
  let accounts = sqlx::query_as!(Account,
    "
SELECT Accounts.id, Accounts.name, Accounts.type AS t
  FROM Accounts
WHERE Accounts.bookkeeping_id = $1
    ",
    bookkeeping.id,
  )
    .fetch_all(&state.db)
    .await?
  ;
  // Then we sort them by account type
  let mut accounts_by_type = std::collections::HashMap::<String,Vec<Account>>::new();
  for account in accounts {
    match accounts_by_type.get_mut(&account.t) {
      Some(x) => x.push(account),
      None => { accounts_by_type.insert(account.t.clone(), vec![account]); },
    }
  }

  html(Index{
    bookkeeping_name: bookkeeping.name,
    imported_account_changes,
    accounts_by_type,
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
      verify_method_path_end(&path_vec, &req, &Method::GET)?;
      index(
        state,
        req,
        session,
        bookkeeping,
      ).await
    },
    _ => Err(Error::path_not_found(&req)),
  }
}

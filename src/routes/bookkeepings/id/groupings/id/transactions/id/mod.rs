use super::*;

mod account_changes;

#[derive(Debug)]
struct Account{
  id: i64,
  name: String,
  t: String,
}
#[derive(Debug)]
struct AccountChange {
  id: i64,
  account_name: String,
  message: String,
  date: Date,
  amount: Decimal,
}
#[derive(Debug, Template)]
#[template(path = "bookkeepings/id/groupings/id/transactions/id/index.html")]
struct Index {
  name: String,
  grouping_name: String,
  bookkeeping_name: String,
  date: Date,
  sum: Decimal,
  accounts_by_type: std::collections::HashMap<String, Vec<Account>>,
  account_changes: Vec<AccountChange>,
  created: Created,
}
#[derive(Debug, Deserialize)]
struct Created {
  new_account_change: Option<i64>,
}
impl Created {
  fn equals_account_change(&self, id: &i64) -> bool {
    self.new_account_change == Some(*id)
  }
}
async fn index(
  state: &'static State,
  req: Request,
  session: SessionData,
  bookkeeping: Bookkeeping,
  grouping: Grouping,
  transaction: TransactionSummary,
  created: Created,
) -> Result<Response, Error> {
  // Then get all the account changes in the transaction
  let account_changes = sqlx::query_as!(AccountChange,
    "
SELECT AccountChanges.id, Accounts.name AS account_name, AccountChanges.message,
    AccountChanges.day AS date, AccountChanges.amount
  FROM AccountChanges
  INNER JOIN Accounts ON AccountChanges.account_id = Accounts.id
WHERE AccountChanges.transaction_id = $1
ORDER BY AccountChanges.day
    ",
    transaction.id,
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
    grouping_name: grouping.name,
    name: transaction.name,
    date: transaction.date,
    sum: transaction.sum,
    account_changes,
    created,
    accounts_by_type,
  }.render()?)
}
pub async fn route(
  state: &'static State,
  req: Request,
  mut path_vec: Vec<String>,
  session: SessionData,
  bookkeeping: Bookkeeping,
  grouping: Grouping,
  transaction_id: i64,
) -> Result<Response, Error> {
  // Query out the transaction summary
  let transaction = sqlx::query_as!(TransactionSummary,
    "
SELECT Transactions.id, Transactions.name, Transactions.day AS \"date\",
    COALESCE(SUM(AccountChanges.amount), 0) AS \"sum!\"
  FROM Transactions
  LEFT JOIN AccountChanges ON AccountChanges.transaction_id = Transactions.id
WHERE Transactions.id = $1
GROUP BY Transactions.id, Transactions.name, Transactions.day
ORDER BY Transactions.day
    ",
    transaction_id,
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
      index(
        state,
        req,
        session,
        bookkeeping,
        grouping,
        transaction,
        created,
      ).await
    },
    Some("account-changes") => account_changes::route(
      state,
      req,
      path_vec,
      session,
      bookkeeping,
      grouping,
      transaction,
    ).await,
    _ => Err(Error::path_not_found(&req)),
  }
}

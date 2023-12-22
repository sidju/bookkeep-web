use super::*;

#[derive(Debug, Deserialize)]
struct NewAccountChange {
  account: i64,
  date: Date,
  message: String,
  amount: Decimal,
}
async fn index_post(
  state: &'static State,
  mut req: Request,
  session: SessionData,
  bookkeeping: Bookkeeping,
  grouping: Grouping,
  transaction: TransactionSummary,
) -> Result<Response, Error> {
  // Parse out the new transaction
  let new_account_change: NewAccountChange = parse_body_urlencoded(
    &mut req,
    state.max_content_len,
  ).await?;
  // Insert into database
  let created = sqlx::query!(
    "
INSERT INTO AccountChanges(account_id, day, message, amount, transaction_id) VALUES($1,$2,$3,$4,$5) RETURNING id
    ",
    new_account_change.account,
    new_account_change.date,
    new_account_change.message,
    new_account_change.amount,
    transaction.id,
  )
    .fetch_one(&state.db)
    .await?
    .id
  ;
  // Redirect to parent with created as query param
  see_other(&format!("../?new_account_change={created}"))
}
pub async fn route(
  state: &'static State,
  req: Request,
  mut path_vec: Vec<String>,
  session: SessionData,
  bookkeeping: Bookkeeping,
  grouping: Grouping,
  transaction: TransactionSummary,
) -> Result<Response, Error> {
  match path_vec.pop().as_deref() {
    None => permanent_redirect(&format!("{}/", req.uri().path())),
    Some("") => {
      verify_method_path_end(&path_vec, &req, &Method::POST)?;
      index_post(
        state,
        req,
        session,
        bookkeeping,
        grouping,
        transaction,
      ).await
    },
    _ => Err(Error::path_not_found(&req)),
  }
}

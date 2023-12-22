use super::*;

mod id;

#[derive(Debug, Deserialize)]
struct NewTransaction {
  name: String,
  date: Date,
}
async fn index_post(
  state: &'static State,
  mut req: Request,
  session: SessionData,
  bookkeeping: Bookkeeping,
  grouping: Grouping,
) -> Result<Response, Error> {
  // Parse out the new transaction
  let new_transaction: NewTransaction = parse_body_urlencoded(
    &mut req,
    state.max_content_len,
  ).await?;
  // Insert into database
  let created = sqlx::query!(
    "
INSERT INTO Transactions(name, day, grouping_id) VALUES($1,$2,$3) RETURNING id
    ",
    new_transaction.name,
    new_transaction.date,
    grouping.id,
  )
    .fetch_one(&state.db)
    .await?
    .id
  ;
  // Redirect to parent with created as query param
  see_other(&format!("../?new_transaction={created}"))
}
pub async fn route(
  state: &'static State,
  req: Request,
  mut path_vec: Vec<String>,
  session: SessionData,
  bookkeeping: Bookkeeping,
  grouping: Grouping,
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
      ).await
    },
    Some(id) => id::route(state, req, path_vec, session, bookkeeping, grouping, id.parse()?).await,
    _ => Err(Error::path_not_found(&req)),
  }
}

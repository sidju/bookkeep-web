use super::*;

mod id;

#[derive(Debug)]
struct GroupingSummary {
  id: i64,
  name: String,
  movement: Decimal,
}
#[derive(Debug, Template)]
#[template(path = "bookkeepings/id/groupings/index.html")]
struct Index {
  groupings: Vec<GroupingSummary>,
}

async fn index(
  state: &'static State,
  req: Request,
  session: SessionData,
  bookkeeping_id: i64,
) -> Result<Response, Error> {
  let g = sqlx::query_as!(GroupingSummary,
    "
SELECT Groupings.id, Groupings.name, COALESCE(SUM(CASE
    WHEN AccountChanges.amount > 0 THEN AccountChanges.amount
    ELSE 0
  END), 0) AS \"movement!\"
  FROM Groupings
  LEFT JOIN Transactions ON Transactions.grouping_id = Groupings.id
  LEFT JOIN AccountChanges ON AccountChanges.transaction_id = Transactions.id
WHERE Groupings.bookkeeping_id = $1
GROUP BY Groupings.id
    ",
    bookkeeping_id,
  )
    .fetch_all(&state.db)
    .await?
  ;

  html(Index{
    groupings: g,
  }.render()?)
}
#[derive(Debug, Deserialize)]
struct NewGrouping {
  name: String,
}
async fn index_post(
  state: &'static State,
  mut req: Request,
  session: SessionData,
  bookkeeping_id: i64,
) -> Result<Response, Error> {
  // Parse out the new grouping
  let new_grouping: NewGrouping = parse_body_urlencoded(
    &mut req,
    state.max_content_len,
  ).await?;
  // Insert into database
  let created = sqlx::query!(
    "INSERT INTO Groupings(name, bookkeeping_id) VALUES($1, $2) RETURNING id",
    new_grouping.name,
    bookkeeping_id,
  )
    .fetch_one(&state.db)
    .await
    // TODO convert
    // if is_unique_violation mark duplicate
    // if foreign_key_violation mark bad request
    ?
    .id
  ;
  // Redirect to parent with newly created grouping marked
  see_other(&format!("../?new_grouping={created}"))
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
    Some(id) => id::route(state, req, path_vec, session, bookkeeping, id.parse()?).await
  }
}

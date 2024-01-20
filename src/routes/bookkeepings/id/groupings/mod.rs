use super::*;

mod id;

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
    // Convert name conflict to readable user error
    .map_err(|e| -> Error { match e {
      sqlx::Error::Database(ref dbe) if dbe.is_unique_violation() => {
        ClientError::AlreadyExists(format!(
          "A grouping by name {} already exists in this bookkeeping.",
          new_grouping.name,
        )).into()
      },
      e => e.into(),
    }})
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
        &Method::POST => index_post(state, req, session, bookkeeping.id).await,
        _ => Err(Error::method_not_found(&req)),
      }
    },
    Some(id) => id::route(state, req, path_vec, session, bookkeeping, id.parse()?).await
  }
}

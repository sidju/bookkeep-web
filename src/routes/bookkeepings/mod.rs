use super::*;

mod id;

#[derive(Debug)]
struct Bookkeeping {
  id: i64,
  name: String,
  owner: String,
}
#[derive(Template)]
#[template(path = "bookkeepings/index.html")]
struct Index {
  email: String,
  bookkeepings: Vec<Bookkeeping>,
}

async fn index(
  state: &'static State,
  session: SessionData,
) -> Result<Response, Error> {
  // Get relevant data in a way that effectively validates permissions
  let bookkeepings = sqlx::query_as!(Bookkeeping,
    "
SELECT Bookkeepings.id, Bookkeepings.name, Users.email AS owner
  FROM Bookkeepings
  LEFT JOIN UsersBookkeepingsAccess ON Bookkeepings.id = bookkeeping_id
  JOIN Users ON Users.id = owner_id
WHERE bookkeepings.owner_id = $1 OR UsersBookkeepingsAccess.user_id = $1
ORDER BY bookkeepings.id DESC
    ",
    session.user_id,
  )
    .fetch_all(&state.db)
    .await?
  ;

  // Render and return
  html(Index{
    email: session.email,
    bookkeepings,
  }.render()?)
}
#[derive(Debug,Deserialize)]
struct NewBookkeeping{
  name: String,
}
#[derive(Template)]
#[template(path = "bookkeepings/bookkeeping-entry.part.html")]
struct IndexPost {
  b: Bookkeeping,
}
async fn index_post(
  state: &'static State,
  mut req: Request,
  session: SessionData,
) -> Result<Response, Error> {
  // Parse out the submitted new bookkeeping
  let new_bookkeeping: NewBookkeeping = parse_body_urlencoded(
    &mut req,
    state.max_content_len,
  ).await?;

  // No validation needed, invalid data can't be represented
  // Insert into database
  let created = sqlx::query_as!(Bookkeeping,
    "INSERT INTO Bookkeepings(name, owner_id) VALUES($1, $2) RETURNING id, name, $3 AS \"owner!\"",
    new_bookkeeping.name,
    session.user_id,
    session.email,
  )
    .fetch_one(&state.db)
    .await
    .map_err(|e| -> Error { match e {
      sqlx::Error::Database(ref dbe) if dbe.is_unique_violation() => {
        ClientError::AlreadyExists(format!(
          "A Bookkeeping by name {} already exists.",
          new_bookkeeping.name,
        )).into()
      },
      sqlx::Error::Database(ref dbe) if dbe.is_check_violation() => {
        ClientError::InvalidData(
          "A Bookkeeping name must contain at least one character.".to_string(),
        ).into()
      },
      e => e.into(),
    }})
    ?
  ;

  // Render and return
  html(IndexPost{
    b: created,
  }.render()?)
}

pub async fn route(
  state: &'static State,
  req: Request,
  mut path_vec: Vec<String>,
) -> Result<Response, Error> {
  // Get out the cookies
  // (For a backend using cookies more than this one, hand in the cookies var to
  // the handlers to provide them access (and perhaps use the `cookie` crate to
  // parse it instead of doing it manually)
  let cookies = parse_cookies(&req)?;

  // Check for session
  let session = match cookies.get("session") {
    Some(id) => {
      // Verify that the id we got is a valid session
      match sqlx::query_as!(SessionData,
        "SELECT session_id, user_id, email
           FROM Sessions
           JOIN Users ON Users.id = Sessions.user_id
         WHERE session_id = $1",
         id,
      )
        .fetch_optional(&state.db)
        .await?
      {
        Some(x) => x,
        None => { return start_oidc_login_flow(state).await; },
      }
    } 
    None => {
      return start_oidc_login_flow(state).await;
    } 
  };

  match path_vec.pop().as_deref() {
    // Means a missing trailing slash, redirect to with slash
    None => permanent_redirect(&format!("{}/", req.uri().path())),
    Some("") => {
      verify_path_end(&path_vec, &req)?;
      match req.method() {
        &Method::GET => index(state, session).await,
        &Method::POST => index_post(state, req, session).await,
        _ => Err(Error::method_not_found(&req)),
      }
    },
    // Parse the path into an integer id and keep routing
    Some(id) => id::route(state, req, path_vec, session, id.parse()?).await,
  }
}

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
  req: Request,
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
    ",
    session.user_id,
  )
    .fetch_all(&state.db)
    .await?
  ;
  println!("{bookkeepings:?}");

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
  let created = sqlx::query!(
    "INSERT INTO Bookkeepings(name, owner_id) VALUES($1, $2) RETURNING id",
    new_bookkeeping.name,
    session.user_id,
  )
    .fetch_one(&state.db)
    .await
    // TODO Convert sqlx::Error::Database(e)
    //   if e.is_unique_violation() to duplicate entry
    //   if e.is_foreign_key_violation() to something?
    ?
    .id
  ;

  // Return a the created object
  add_header(
    set_status(
      empty(),
      hyper::StatusCode::SEE_OTHER,
    ),
    hyper::header::LOCATION,
    hyper::header::HeaderValue::from_str(&format!("{}/", created))?,
  )
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
      match req.method() {
        &Method::GET => {
          verify_path_end(&path_vec, &req)?;
          index(state, req, session).await
        },
        &Method::POST => {
          verify_path_end(&path_vec, &req)?;
          index_post(state, req, session).await
        },
        _ => Err(Error::method_not_found(&req)),
      }
    },
    // Parse the path into an integer id and keep routing
    Some(id) => id::route(state, req, path_vec, session, id.parse()?).await,
  }
}

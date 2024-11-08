mod config;
use axum::body::Body;
use axum::http::Request;
use axum::middleware::{from_fn, Next};
use axum::response::Response;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use base64::engine::general_purpose;
use base64::Engine;
pub use config::config;

mod git;

mod error;
pub use error::{Error, Result};

use axum::async_trait;
use axum::extract::{FromRequestParts, Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{prelude::*, Duration};
use git::Report;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use tokio::{net::TcpListener, runtime::Runtime};
use tower_http::cors::{Any, CorsLayer};
use walkdir::WalkDir;

use std::sync::OnceLock;
use std::{
    fs, mem,
    sync::{Arc, Mutex},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Assignment {
    pub name: String,
    pub filepath: String,
    pub deadline: DateTime<Utc>,
    pub grader: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Submission {
    pub email: String,
    pub id: i64,
    pub name: String,
    pub section: String,
    pub link: String,
}

#[derive(Clone)]
pub struct AppState {
    pub report: Arc<Mutex<Vec<Report>>>,
}

pub fn assignments() -> &'static Vec<Assignment> {
    static INSTANCE: OnceLock<Vec<Assignment>> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        // collect assignments
        let assignments = fs::read_to_string(config().assignments.clone())
            .expect("expected assignments.json file!");
        let assignments: Vec<Assignment> = serde_json::from_str(assignments.as_ref()).unwrap();
        assignments
    })
}

pub fn submissions() -> &'static Vec<Submission> {
    static INSTANCE: OnceLock<Vec<Submission>> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        // collect submissions
        let mut reader = csv::Reader::from_path(config().submissions.clone())
            .expect("error processing submissions.csv file!");
        let mut submissions: Vec<Submission> = vec![];
        for result in reader.deserialize() {
            let submission: Submission = result.expect("failed to deserialize the submission");
            submissions.push(submission);
        }
        submissions
    })
}

#[tokio::main]
async fn main() {
    let assignments = assignments();
    let submissions = submissions();

    let state = AppState {
        report: Arc::new(Mutex::new(vec![])),
    };
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_origin(Any)
        .allow_headers(Any);
    let app = Router::new()
        .route("/", get(get_submissions))
        .route("/files/:id", get(get_files))
        .route("/login", post(login))
        .route("/reports/:id/assignments/:name", get(get_pdf))
        .layer(from_fn(resolve_ctx))
        .layer(cors)
        .with_state(state.clone());

    // in an infinite loop, download all git repos
    let state_clone = Arc::clone(&state.report);
    let rt = Runtime::new().unwrap();
    rt.spawn(async move {
        loop {
            let report = git::report(assignments, submissions).unwrap();
            // let report = git::report(&assignments, &submissions).unwrap();
            let mut state = state_clone.lock().unwrap();
            let _ = mem::replace(&mut *state, report);
            println!("report updated");
            drop(state);
        }
    });

    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn get_submissions(State(state): State<AppState>) -> Json<Vec<Report>> {
    let current = state.report.lock().unwrap();
    let copy = (*current).clone();
    axum::Json(copy)
}

async fn get_files(Path(id): Path<String>) -> Json<Vec<String>> {
    let root = format!("{}/{}/", config().downloads.clone(), id);
    let entries: Vec<String> = WalkDir::new(&root)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().display().to_string())
        .map(|e| e.replace(&root, ""))
        .filter_map(|e| (!e.is_empty()).then_some(e))
        .filter_map(|e| (!e.starts_with('.')).then_some(e))
        .collect();

    axum::Json(entries)
}

#[derive(Serialize, Deserialize, Debug)]
struct Claim {
    exp: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct LoginInfo {
    password: String,
}

async fn login(Json(login_info): Json<LoginInfo>) -> Json<String> {
    println!("{} == {}", login_info.password, config().password);
    if login_info.password != config().password {
        return Json("incorrect!".to_string());
    }
    let claim = Claim {
        exp: (Utc::now() + Duration::hours(3)).timestamp() as usize,
    };
    let token = encode(
        &Header::default(),
        &claim,
        &EncodingKey::from_secret(config().jwt_key.as_bytes()),
    );
    match token {
        Ok(v) => Json(v),
        // TODO: Handle error better. May be all endpoints return Result<>
        Err(_) => Json("There was an error logging in".to_string()),
    }
}

#[derive(Clone, Debug)]
pub struct Context {
    authenticated: bool,
}

impl Context {
    pub fn new(authenticated: bool) -> Self {
        Self { authenticated }
    }
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Context {
    type Rejection = Error;
    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> std::prelude::v1::Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Result<Context>>()
            .ok_or(Error::JWT)?
            .clone()
    }
}

async fn resolve_ctx(
    bearer: Option<TypedHeader<Authorization<Bearer>>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response> {
    let bearer = match bearer {
        None => return Ok(next.run(request).await),
        Some(TypedHeader(Authorization(v))) => v,
    };
    let _ = decode::<Claim>(
        bearer.token(),
        &DecodingKey::from_secret(config().jwt_key.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| Error::JWT)?;
    let context = Context::new(true);
    request
        .extensions_mut()
        .insert::<Result<Context>>(Ok(context));
    Ok(next.run(request).await)
}

async fn get_pdf(Path((id, assignment)): Path<(usize, String)>, context: Context) -> Json<String> {
    if !context.authenticated {
        return Json("sorry, unauthenticated!".to_string());
    }
    let assignments = assignments();
    for item in assignments {
        if item.name == assignment {
            let path = format!("{}/{}/{}", config().downloads, id, item.filepath);
            println!("{path:?}");
            return match fs::read(path) {
                Ok(v) => Json(general_purpose::STANDARD.encode(v)),
                Err(e) => return Json(format!("There was an error reading file {e:?}")),
            };
        }
    }
    Json("Something went wrong!".to_string())
}

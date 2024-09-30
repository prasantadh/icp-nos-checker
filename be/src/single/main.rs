mod config;
pub use config::config;

mod git;

mod error;
pub use error::{Error, Result};

use axum::{extract::State, routing::get, Router};
use chrono::prelude::*;
use git::Report;
use serde::{Deserialize, Serialize};
use tokio::{net::TcpListener, runtime::Runtime};

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

#[tokio::main]
async fn main() {
    // collect submissions
    let mut reader = csv::Reader::from_path(config().submissions.clone()).unwrap();
    let mut submissions: Vec<Submission> = vec![];
    for result in reader.deserialize() {
        let submission: Submission = result.expect("failed to deserialize the submission");
        submissions.push(submission);
    }

    // collect assignments
    let assignments =
        fs::read_to_string(config().assignments.clone()).expect("expected assignments.json file!");
    let assignments: Vec<Assignment> = serde_json::from_str(assignments.as_ref()).unwrap();

    let state = AppState {
        report: Arc::new(Mutex::new(vec![])),
    };
    let app = Router::new()
        .route("/", get(get_submissions))
        .with_state(state.clone());

    // in an infinite loop, download all git repos
    let state_clone = Arc::clone(&state.report);
    let rt = Runtime::new().unwrap();
    rt.spawn(async move {
        loop {
            let report = git::report(&assignments, &submissions).unwrap();
            let mut state = state_clone.lock().unwrap();
            mem::replace(&mut *state, report);
            println!("report updated");
            drop(state);
        }
    });

    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn get_submissions(State(state): State<AppState>) -> String {
    let current = state.report.lock().unwrap();
    format!("{current:#?}")
}

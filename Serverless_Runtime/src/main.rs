
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use deno_core::{
    error::AnyError,
    op,
    serde_v8,
    v8,
    JsRuntime,
    OpState,
    RuntimeOptions,
};

use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;
use tracing::{error, info};
use tracing_subscriber::prelude::*;

type DB = Arc<Mutex<Connection>>;

type FunctionMap = HashMap<String, (String, DB)>;

type AppState = Arc<Mutex<FunctionMap>>;

type ApiError = (StatusCode, String);

// --------------------------------------------------
// HTTP HANDLERS
// --------------------------------------------------

async fn handle_root() -> &'static str {
    "Hello rustau!"
}

// POST /fn/:name
async fn handle_fn_submit(
    State(state): State<AppState>,
    Path(name): Path<String>,
    body: String,
) -> Result<String, ApiError> {

    validate_function_name(&name)?;

    if body.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Function body cannot be empty".to_string(),
        ));
    }

    let db_file = format!("{name}.db");

    let db = Connection::open(&db_file)
        .map_err(internal_error)?;

    db.busy_timeout(Duration::from_secs(5))
        .map_err(internal_error)?;

    db.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS kv (
            key TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL
        );
        "
    )
    .map_err(internal_error)?;

    let db = Arc::new(Mutex::new(db));

    {
        let mut functions = state.lock()
            .map_err(internal_error)?;

        functions.insert(
            name.clone(),
            (body, db),
        );
    }

    info!("Registered function: {name}");

    Ok(format!("Function '{name}' registered successfully"))
}

// GET /fn/:name
async fn handle_fn_execute(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Value>, ApiError> {

    validate_function_name(&name)?;

    let (fn_body, db) = {
        let functions = state.lock()
            .map_err(internal_error)?;

        functions
            .get(&name)
            .cloned()
            .ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    format!("Unknown function: {name}"),
                )
            })?
    };

    info!("Executing function: {name}");

    let result = tokio::task::spawn_blocking(move || {
        run_js(&name, &fn_body, db)
    })
    .await
    .map_err(internal_error)??;

    Ok(Json(result))
}

// --------------------------------------------------
// JAVASCRIPT OPERATIONS
// --------------------------------------------------

#[op]
fn op_log(state: &mut OpState, msg: String) {
    let name = state.borrow::<String>();

    info!("[{}] {}", name, msg);
}

#[op]
fn op_kv_set(
    state: &mut OpState,
    key: String,
    value: String,
) -> Result<(), AnyError> {

    let db = state.borrow::<DB>().clone();

    let db = db.lock()
        .map_err(|e| AnyError::msg(e.to_string()))?;

    db.execute(
        "
        INSERT INTO kv (key, value)
        VALUES (?1, ?2)
        ON CONFLICT(key)
        DO UPDATE SET value = excluded.value
        ",
        [&key, &value],
    )?;

    Ok(())
}

#[op]
fn op_kv_get(
    state: &mut OpState,
    key: String,
) -> Result<Option<String>, AnyError> {

    let db = state.borrow::<DB>().clone();

    let db = db.lock()
        .map_err(|e| AnyError::msg(e.to_string()))?;

    let result = db
        .query_row(
            "SELECT value FROM kv WHERE key = ?1",
            [&key],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    Ok(result)
}

// --------------------------------------------------
// JAVASCRIPT BOOTSTRAP
// --------------------------------------------------

const RUNTIME_BOOTSTRAP: &str = r#"

globalThis.console = {
    log: (...args) => {
        Deno.core.opSync(
            "op_log",
            args.map(String).join(" ")
        );
    }
};

globalThis.set = (key, value) => {

    const serialized = JSON.stringify(value);

    if (serialized === undefined) {
        throw new TypeError(
            "Value cannot be serialized to JSON"
        );
    }

    Deno.core.opSync(
        "op_kv_set",
        String(key),
        serialized
    );

    return value;
};

globalThis.get = (key) => {

    const value = Deno.core.opSync(
        "op_kv_get",
        String(key)
    );

    return value === null
        ? null
        : JSON.parse(value);
};

"#;

// --------------------------------------------------
// JAVASCRIPT EXECUTION
// --------------------------------------------------

fn run_js(
    name: &str,
    body: &str,
    db: DB,
) -> Result<Value, ApiError> {

    let mut runtime = JsRuntime::new(RuntimeOptions {

        extensions: vec![
            deno_core::Extension::builder()
                .ops(vec![
                    op_log::decl(),
                    op_kv_set::decl(),
                    op_kv_get::decl(),
                ])
                .js(vec![
                    ("[runtime]", RUNTIME_BOOTSTRAP)
                ])
                .build()
        ],

        ..Default::default()
    });

    // Initialize runtime state.
    {
        let state = runtime.op_state();

        let mut state = state.borrow_mut();

        state.put::<String>(name.to_owned());
        state.put::<DB>(db);
    }

    // Execute JavaScript.
    let last_value = runtime
        .execute_script(name, body)
        .map_err(|err| {
            error!("JavaScript error in {name}: {err}");

            (
                StatusCode::BAD_REQUEST,
                "JavaScript execution failed".to_string(),
            )
        })?;

    // Convert V8 result into serde_json::Value.
    let scope = &mut runtime.handle_scope();

    let local = v8::Local::new(scope, last_value);

    if local.is_undefined() {
        return Ok(Value::Null);
    }

    let result = serde_v8::from_v8::<Value>(
        scope,
        local,
    )
    .map_err(|err| {
        error!("Serialization error in {name}: {err}");

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Result serialization failed".to_string(),
        )
    })?;

    info!("Function {name} completed");

    Ok(result)
}

// --------------------------------------------------
// VALIDATION
// --------------------------------------------------

fn validate_function_name(name: &str) -> Result<(), ApiError> {

    if name.is_empty()
        || name.len() > 64
        || !name.chars().all(|c| c.is_ascii_alphabetic())
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid function name. Use 1-64 ASCII letters only."
                .to_string(),
        ));
    }

    Ok(())
}

// --------------------------------------------------
// ERROR HANDLING
// --------------------------------------------------

fn internal_error(
    err: impl std::fmt::Display,
) -> ApiError {

    error!("Internal error: {err}");

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal server error".to_string(),
    )
}

// --------------------------------------------------
// SERVER
// --------------------------------------------------

#[tokio::main]
async fn main() {

    register_trace_stdout_listener();

    let state: AppState = Arc::new(
        Mutex::new(HashMap::new())
    );

    let app = Router::new()
        .route("/", get(handle_root))
        .route(
            "/fn/:name",
            get(handle_fn_execute)
                .post(handle_fn_submit),
        )
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .with_state(state);

    let addr = std::net::SocketAddr::from((
        std::net::Ipv4Addr::UNSPECIFIED,
        8080,
    ));

    info!("Server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind server");

    axum::Server::from_tcp(
        listener.into_std().expect("Failed to create TCP listener")
    )
    .expect("Failed to create Axum server")
    .serve(app.into_make_service())
    .await
    .expect("Server error");
}

// --------------------------------------------------
// LOGGING
// --------------------------------------------------

fn register_trace_stdout_listener() {

    let env_filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(
            tracing::metadata::LevelFilter::INFO.into()
        )
        .from_env_lossy();

    let filter_layer =
        tracing_subscriber::fmt::layer()
            .with_filter(env_filter);

    tracing_subscriber::registry()
        .with(filter_layer)
        .init();
}

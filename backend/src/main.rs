use clap::Parser;
use std::net::SocketAddr;
use std::path::Path;
use tracing_subscriber::EnvFilter;

mod data;
mod engine;
mod learning;
mod server;

use data::schema::open_database;
use data::weights::load_latest_weights;
use engine::eval::EvalWeights;
use server::ws::create_router;

#[derive(Parser, Debug)]
#[command(name = "nwave-chess", about = "Self-learning chess engine")]
struct Args {
    /// Port to listen on
    #[arg(long, default_value = "3000")]
    port: u16,

    /// Path to SQLite database file
    #[arg(long, default_value = "data/nwave-chess.db")]
    db_path: String,

    /// Maximum search depth for the engine
    #[arg(long, default_value = "6")]
    search_depth: u8,

    /// Log detailed search information (depth, candidates, PV) to the terminal
    #[arg(long, default_value_t = false)]
    search_log: bool,

    /// Path to frontend static files
    #[arg(long, default_value = "frontend/dist")]
    frontend_dir: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("nwave_chess=info".parse().unwrap()),
        )
        .init();

    let args = Args::parse();

    // Ensure database directory exists.
    if let Some(parent) = Path::new(&args.db_path).parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).expect("Failed to create database directory");
            tracing::info!("Created database directory: {}", parent.display());
        }
    }

    // Initialize SQLite database and schema.
    let conn = open_database(&args.db_path).expect("Failed to open database");
    tracing::info!("Database initialized at {}", args.db_path);

    // Load weights from database or use defaults.
    let weights = match load_latest_weights(&conn) {
        Ok(Some((version, weights))) => {
            tracing::info!("Loaded weights version {} from database", version);
            weights
        }
        Ok(None) => {
            tracing::info!("No saved weights found, using default weights");
            EvalWeights::default_weights()
        }
        Err(e) => {
            tracing::warn!("Failed to load weights: {}, using defaults", e);
            EvalWeights::default_weights()
        }
    };

    // Build the router.
    let app = create_router(weights, args.search_depth, args.search_log, &args.frontend_dir);

    // Start the server.
    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    tracing::info!("nwave-chess server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}

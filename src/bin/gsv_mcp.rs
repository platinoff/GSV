//! `gsv-mcp` — stdio MCP server (`gsv_mcp_openbot`).
//!
//! JSON-RPC 2.0, one UTF-8 object per line. Tracing goes to stderr so stdout
//! stays a clean MCP stream.
//!
//! ```text
//! target/live/gsv-mcp.exe --repo-root S:/rust/GSV
//! cargo build --bin gsv-mcp && cargo xtask live   # copies debug → live
//! ```

use std::path::PathBuf;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::broadcast;
use tracing_subscriber::EnvFilter;

use gsv::{mcp, AppState};

/// `--repo-root P`, `--data-dir P`, `--help`.
fn parse_args() -> (Option<PathBuf>, Option<PathBuf>) {
    let mut repo_root = None;
    let mut data_dir = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--repo-root" => repo_root = args.next().map(PathBuf::from),
            "--data-dir" => data_dir = args.next().map(PathBuf::from),
            "--help" | "-h" => {
                println!(
                    "Usage: gsv-mcp [--repo-root P] [--data-dir P]\nMCP server id: {}",
                    mcp::SERVER_ID
                );
                std::process::exit(0);
            }
            _ => {}
        }
    }
    (repo_root, data_dir)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn,gsv=info")),
        )
        .init();

    let (repo_root, data_dir) = parse_args();
    let (tx, _rx) = broadcast::channel(32);
    let state = AppState::new(repo_root, data_dir, tx);

    tracing::info!(
        server = mcp::SERVER_ID,
        protocol = mcp::PROTOCOL_VERSION,
        repo_root = %state.repo_root.display(),
        "gsv-mcp stdio ready"
    );

    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();
    let mut stdout = tokio::io::stdout();
    while let Some(line) = lines.next_line().await? {
        if let Some(out) = mcp::handle_line(&state, &line).await {
            stdout.write_all(out.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
    }
    Ok(())
}

use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod pvf;
mod subxt;

#[derive(Parser)]
#[clap(version)]
struct Cli {
    #[clap(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Given the rpc url, fetch all of the parachain validation codes
    /// from the runtime and try to compile them using PVF workers.
    ///
    /// # Example
    ///
    /// ```bash
    /// cargo run --release -- pvf-check --rpc-url "wss://kusama-rpc.polkadot.io:443"
    /// ```
    PvfCheck {
        /// Url for an RPC node to query the relay chain runtime.
        ///
        /// Example:
        /// `wss://kusama-rpc.polkadot.io:443` or `http://localhost:9933/`
        #[clap(long)]
        rpc_url: String,
    },

    // These are needed for pvf workers:
    #[allow(missing_docs)]
    #[clap(name = "prepare-worker", hide = true)]
    PvfPrepareWorker(ValidationWorkerCommand),

    #[allow(missing_docs)]
    #[clap(name = "execute-worker", hide = true)]
    PvfExecuteWorker(ValidationWorkerCommand),
}

#[allow(missing_docs)]
#[derive(Debug, Parser)]
pub struct ValidationWorkerCommand {
    /// The path to the validation host's socket.
    #[arg(long)]
    pub socket_path: String,
    /// Calling node implementation version
    #[arg(long)]
    pub node_impl_version: String,
}

async fn handle_pvf_check(rpc_url: String) -> anyhow::Result<()> {
    let artifacts = PathBuf::from(".artifacts");
    let _ = std::fs::create_dir_all(artifacts.as_path());

    let pvfs_path = artifacts.as_path().join("pvfs");
    let _ = std::fs::create_dir_all(&pvfs_path);

    print!("Fetching PVFs...");
    let pvfs = subxt::fetch_all_pvfs(rpc_url).await?;
    println!(" SUCCESS ({} PVFs)", pvfs.len());

    let validation_host = pvf::setup_pvf_worker(pvfs_path).await;

    for (para_id, pvf) in pvfs {
        print!("Pre-checking 0x{}:", hex::encode(&para_id));
        let duration = pvf::precheck_pvf(validation_host.clone(), pvf).await?;
        println!(" SUCCESS ({}ms)", duration.as_millis());
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    let cli = Cli::parse();

    match cli.commands {
        Commands::PvfCheck { rpc_url } => {
            rt.block_on(handle_pvf_check(rpc_url))?;
        }
        Commands::PvfPrepareWorker(params) => {
            polkadot_node_core_pvf_worker::prepare_worker_entrypoint(
                &params.socket_path,
                Some(&params.node_impl_version),
            );
        }
        Commands::PvfExecuteWorker(_params) => {
            unimplemented!("not needed for pre-checking")
        }
    }

    Ok(())
}

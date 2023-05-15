use futures::channel::oneshot;
use polkadot_node_core_pvf::{Config, PvfPrepData, ValidationHost};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use polkadot_parachain::primitives::ValidationCode;

fn other_io_error(s: String) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, s)
}

pub async fn setup_pvf_worker(pvfs_path: PathBuf) -> ValidationHost {
    let program_path = std::env::current_exe().expect("current_exe failed?");
    // FIXME: support non-default ExecutorParams
    let (validation_host, worker) =
        polkadot_node_core_pvf::start(Config::new(pvfs_path, program_path), Default::default());

    // CURSED
    let _detached_thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(worker);
    });

    validation_host
}

pub async fn precheck_pvf(
    mut validation_host: ValidationHost,
    pvf: ValidationCode,
) -> anyhow::Result<Duration> {
    let raw_validation_code =
        sp_maybe_compressed_blob::decompress(&pvf.0, 12 * 1024 * 1024)?.to_vec();

    let pvf = PvfPrepData::from_code(
        raw_validation_code,
        // FIXME: support non-default ExecutorParams
        Default::default(),
        Duration::from_secs(60),
    );

    let (tx, rx) = oneshot::channel();

    let now = Instant::now();

    validation_host
        .precheck_pvf(pvf.clone(), tx)
        .await
        .map_err(other_io_error)?;

    rx.await?.map_err(|e| other_io_error(format!("{e:?}")))?;

    Ok(now.elapsed())
}

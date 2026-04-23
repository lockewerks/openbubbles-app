use std::{path::Path, sync::LazyLock};

use flexi_logger::{opt_format, Age, Cleanup, Criterion, FileSpec, Logger, Naming, WriteMode};
use log::info;

pub static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    info!("creating runner");
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name("tokio-whatbubbles")
        .enable_all()
        .build()
        .unwrap()
});

pub mod bbhwinfo {
    include!(concat!(env!("OUT_DIR"), "/bbhwinfo.rs"));
}

pub fn init_logger(path: &Path) {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "debug");
    }
    let system = pretty_env_logger::formatted_builder().build();

    let (logger, _) = Logger::try_with_str("debug")
        .expect("No logger?")
        .log_to_file(FileSpec::default().directory(path.join("logs")).suppress_timestamp())
        .append()
        .format(opt_format)
        .cleanup_in_background_thread(false)
        .rotate(
            Criterion::AgeOrSize(Age::Day, 1024 * 1024 * 10),
            Naming::Numbers,
            Cleanup::KeepLogFiles(1),
        )
        .write_mode(WriteMode::BufferAndFlush)
        .build()
        .unwrap();

    let _ = multi_log::MultiLogger::init(vec![Box::new(system), logger], log::Level::Trace);
}

pub mod app;
pub mod events;
pub mod ffi;
pub mod integration;
pub mod storage;

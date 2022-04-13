use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(name = "rabbit-cli")]
#[clap(about = "local document search tool", version="0.5.0", long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// index dirs
    #[clap(arg_required_else_help = true)]
    Index {
        /// the path to index
        dir: String,
    },
    /// search
    Search {
        /// query string
        query: String,
    }
}

fn init_logger(dir: &str) -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender = tracing_appender::rolling::daily(dir, "tracing.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let format = tracing_subscriber::fmt::format()
        .with_level(true)
        .with_target(true)
        .with_timer(tracing_subscriber::fmt::time::time());

    tracing_subscriber::fmt()
        .with_writer(non_blocking) // 写入文件，将覆盖上面的标准输出
        .with_ansi(false)  // 如果日志是写入文件，应将ansi的颜色输出功能关掉
        .event_format(format)
        .init();  // 初始化并将SubScriber设置为全局SubScriber

    _guard
}

fn main() -> Result<()> {
    let data_path = dirs::home_dir().unwrap().join(".rabbit");
    let index_path = data_path.join("index");
    if !index_path.exists() {
        std::fs::create_dir_all(index_path.as_path())?;
    }

    let _guard = init_logger(data_path.to_str().unwrap());
    log::info!("begin");
    let index_server = &mut rabbit::index::IndexServer::new(index_path.to_str().unwrap())?;

    let args = Cli::parse();
    match &args.command {
        Commands::Index { dir } => {
            rabbit::recursive_index(index_server, dir);
        },
        Commands::Search { query} => {
            log::info!("search: {}", query);
            let result = index_server.search(query.to_string()).unwrap();
            log::info!("search: {} finish", query);
            print!("result: {:#?}", result.paths);
        }
    }

    Ok(())
}
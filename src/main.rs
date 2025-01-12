use clap::{CommandFactory, Parser, Subcommand};
mod mutators;
mod run;
mod server;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Parser, Default)]
#[command(
    about = "Bitcoin Core Mutations",
    long_about = "Bticoin Core Mutuaitons is a tool for mutating Bitcoin Core's source code."
)]
struct Args {
    #[clap(subcommand)]
    action: Option<Action>,
}

#[derive(Subcommand)]
enum Action {
    #[clap(name = "mutate", about = "Mutate files")]
    Mutate {
        #[clap(short, long, help = "Files to mutate")]
        files: Vec<String>,
        #[clap(short, long, help = "Server to send mutations")]
        server: String,
        #[clap(long, help = "Token to use for authentication")]
        token: String,
    },
    #[clap(name = "server", about = "Start the server")]
    Server {
        #[clap(long, help = "Host", default_value = "0.0.0.0")]
        host: String,
        #[clap(long, help = "Port", default_value = "8080")]
        port: u16,
        #[clap(long, help = "Redis database", default_value = "127.0.0.1")]
        redis: String,
        #[clap(long = "token", help = "Authorized tokens (owner:token)")]
        tokens: Vec<String>,
    },
    #[clap(name = "run", about = "Run mutations")]
    Run {
        #[clap(short, long, help = "Server to get work from")]
        server: String,
        #[clap(
            short,
            long,
            help = "Path to Bitcoin Core",
            default_value = "/tmp/bitcoin"
        )]
        path: String,
        #[clap(short, long, help = "Build command", default_value = "make -j$(nproc)")]
        build_cmd: String,
        #[clap(
            short,
            long,
            help = "Test command",
            default_value = "make check -j$(expr $(nproc) + 4) && python3 -u test/functional/test_runner.py -j$(expr $(nproc) + 4) -F"
        )]
        test_cmd: String,
        #[clap(long, help = "Token to use for authentication")]
        token: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum MutationStatus {
    Pending,
    Timeout,
    Running,
    Killed,
    NotKilled,
    Ignored,
    Error,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Mutation {
    id: String,
    file: String,
    line: usize,
    patch: String,
    branch: String,
    pr_number: Option<String>,
    status: MutationStatus,
    #[serde(default, with = "time::serde::timestamp::option")]
    start_time: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::timestamp::option")]
    end_time: Option<OffsetDateTime>,
    stderr: Option<String>,
    stdout: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MutationResult {
    mutation_id: String,
    status: MutationStatus,
    stdout: Option<String>,
    stderr: Option<String>,
}

#[actix_web::main]
async fn main() {
    let args = Args::parse();

    ctrlc::set_handler(move || {
        println!("Bye!");
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    match &args.action {
        Some(Action::Mutate {
            server,
            files,
            token,
        }) => {
            if files.is_empty() {
                println!("No files to mutate, please specify some files with --files.");
                return;
            }

            let mutations = mutators::create_mutations(files);
            mutators::send_mutations(server.to_string(), mutations, token)
                .await
                .unwrap();
        }
        Some(Action::Server {
            host,
            port,
            redis,
            tokens,
        }) => {
            server::run(host.clone(), *port, redis.clone(), tokens.clone()).await;
        }
        Some(Action::Run {
            server,
            path,
            build_cmd,
            test_cmd,
            token,
        }) => {
            run::execute_mutations(server, path, build_cmd, test_cmd, token).await;
        }
        None => {
            let mut cmd = Args::command();
            cmd.print_help().unwrap_or(());
        }
    }
}

mod challenge_response;
mod client;
mod io_utils;
mod protocol;
mod server;

use getopts::Options;
use std::env;

fn usage(program: &str, opts: &Options) {
    let brief = format!(
        "Usage: {} [options] server [BINDADDR:]BINDPORT PORT
       {} [options] client PROXY:PORT TARGET-HOST:PORT",
        program, program
    );
    print!("{}", opts.usage(&brief));
    std::process::exit(1);
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].to_owned();

    let mut opts = Options::new();
    opts.optopt("p", "psk", "Preshared Key", "PSK");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            println!("{}", f.to_string());
            return usage(&program, &opts);
        }
    };

    if matches.free.len() != 3 {
        return usage(&program, &opts);
    }

    match matches.free[0].as_str() {
        "server" => server::server(server::ServerConfig {
            psk: matches
                .opt_str("psk")
                .map(|x| x.into_bytes())
                .unwrap_or_else(|| vec![]),
            reverse_port: matches.free[1].to_owned(),
            target: matches.free[2].to_owned(),
        })
        .await
        .unwrap(),
        "client" => client::client(client::ClientConfig {
            psk: matches
                .opt_str("psk")
                .map(|x| x.into_bytes())
                .unwrap_or_else(|| vec![]),
            reverse_server: matches.free[1].to_owned(),
            target: matches.free[2].to_owned(),
        })
        .await
        .unwrap(),
        _ => usage(&program, &opts),
    }
}

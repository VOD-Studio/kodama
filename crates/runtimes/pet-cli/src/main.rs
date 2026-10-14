mod account;
mod live;

use pet_core::execute;
use std::io::{BufRead, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("live") => std::process::exit(live::run(args.iter().any(|a| a == "--account"))),
        Some(cmd) if account::COMMANDS.contains(&cmd) => {
            std::process::exit(account::run(cmd, &args[2..]))
        }
        Some("capabilities") => println!("{}", execute("{\"op\":\"capabilities\"}")),
        Some("help" | "--help" | "-h") => help(),
        Some("version" | "--version" | "-V") => version(),
        None | Some("exec") => exec(),
        Some(other) => {
            eprintln!("unknown command: {other}\nrun `pet-cli help`");
            std::process::exit(2);
        }
    }
}

/// 帮助文本中的能力列表与机器可读发现同源(pet_core::CAPABILITIES)。
fn help() {
    println!("usage: pet-cli [exec|live|capabilities|version|help|login|logout|status|act]\n");
    println!(
        "  exec          read one JSON command per line on stdin, write one JSON result per line"
    );
    println!("  live          interactive terminal pet (non-TTY: one JSON status line)");
    println!("  live --account  your account pet; f feed, p play, r rest, h heal");
    println!("  login         sign in to a pet service: --service URL or PET_SERVICE");
    println!("  logout        sign out and forget the saved credentials");
    println!("  status        your account pet as one JSON line");
    println!("  act ACTION    feed|rest|play|heal|wait|evolve BRANCH [--request-id ID]");
    println!("  capabilities  print the machine-readable capability catalog");
    println!("  version       print the release version and the rules version\n");
    println!("ops accepted by exec (rules v{}):", pet_core::RULES_VERSION);
    for c in pet_core::CAPABILITIES {
        let errors: Vec<String> = c.errors.iter().map(|e| format!("{e:?}")).collect();
        let errors = if errors.is_empty() {
            "-".to_string()
        } else {
            errors.join(", ")
        };
        println!(
            "  {:<13} {}{}\n  {:<13} errors: {}",
            c.op,
            c.summary,
            if c.mutates_state {
                " [mutates state]"
            } else {
                ""
            },
            "",
            errors
        );
    }
}

/// 发布归档不带包管理器的元数据，版本只能从二进制本身读到。
fn version() {
    println!(
        "pet-cli {} (rules v{})",
        env!("CARGO_PKG_VERSION"),
        pet_core::RULES_VERSION
    );
}

/// 每行一个 JSON 命令,每行输出一个 JSON 结果。
fn exec() {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line.expect("stdin");
        if line.trim().is_empty() {
            continue;
        }
        writeln!(out, "{}", execute(&line)).expect("stdout");
    }
}

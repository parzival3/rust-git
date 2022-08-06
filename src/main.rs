use std::env;
use std::fs;
use std::process;

fn print_usage() {
    println!("usage: codecrafters-git-rust <command>");
    println!("Available commands: ");
    println!("\t [init]: initialize git repository");
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        if args[1] == "init" {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/master\n").unwrap();
            println!("Initialized git directory")
        } else {
            println!("unknown command: {}", args[1]);
            print_usage();
            process::exit(-1);
        }
    } else {
        print_usage();
        process::exit(-1);
    }
}

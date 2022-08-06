use std::env;
use std::process;

fn print_usage() {
    println!("usage: codecrafters-git-rust <command>");
    println!("Available commands: ");
    println!("\t [init]: initialize git repository");
    println!("\t [cat-file]: read a blob of data from the `object` directory.");
    println!(
        "\t\t ars: [-p] [sha]: output the content of the object with `sha` to the standard output."
    );
}

mod plumming {
    use std::fs;
    pub const GIT_OBJECTS: &str = ".git/objects";

    // init a git repository by creating the directory structure found in .git
    pub fn init() {
        fs::create_dir(".git").unwrap();
        fs::create_dir(".git/objects").unwrap();
        fs::create_dir(".git/refs").unwrap();
        fs::write(".git/HEAD", "ref: refs/heads/master\n").unwrap();
        println!("Initialized git directory")
    }

    pub mod cat {
        use super::*;
        use std::io::prelude::*;
        use flate2::read::ZlibDecoder;
        // implements the cat-file pretty-print command of git
        // as input it accepts a sha1 String representing the sha of an object stored in the object directory
        // and it returns the content of that object as String.
        pub fn sha_obect_to_string(sha_object: &String) -> std::io::Result<String> {
            // Git stores its obects based on the hash.
            // The first two hex numbers are the directory in which they are stored and the last is the actual name of the file
            // found int he object directory.
            let (dir_name, file_name) = sha_object.split_at(2);
            let full_path = GIT_OBJECTS.to_string() + "/" + dir_name + "/" + file_name;
            let file_content = fs::read(full_path)?;
            let mut z = ZlibDecoder::new(&file_content[..]);
            let mut s = String::new();
            z.read_to_string(&mut s)?;
            Ok(s)
        }

        // This function takes a `sha` of an object and prints the content of the
        // file with the same `sha`.
        pub fn pretty_print(sha_object: &String) -> std::io::Result<()> {
            let file_content = sha_obect_to_string(sha_object)?;
            println!("{}", file_content);
            Ok(())
        }
    }

    pub fn cat_file(args: &[String]) -> Result<(), String> {
        if args[0] == "-p" && args.len() == 2 {
            match cat::pretty_print(&args[1]) {
                Ok(_) => Ok(()),
                Err(e) => Err(format!(
                    "Error: cat-file -p command failed with error: '{}'",
                    e
                )),
            }
        } else {
            Err("Error: args[0] {}, not a valid cat-file command".to_string())
        }
    }
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        if args[1] == "init" {
            plumming::init();
        } else if args[1] == "cat-file" {
            match plumming::cat_file(&args[2..]) {
                Ok(_) => process::exit(0),
                Err(s) => {
                    println!("{}", s);
                    process::exit(-1)
                }
            }
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

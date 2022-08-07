use std::env;
use std::process;

fn print_usage() {
    println!("usage: codecrafters-git-rust <command>");
    println!("Available commands: ");
    println!("\t [init]: initialize git repository");
    println!("\t [cat-file]: read a blob of data from the `object` directory.");
    println!(
        "\t\t args: [-p] [sha]: output the content of the object with `sha` to the standard output."
    );
    println!("\t [hash-object]: computes object ID and optionally creates a blob from a file.");
    println!("\t\t args: [-w] [file-name]: actually write the object into the object database.");
}

mod plumming {
    use std::fs;
    pub const GIT_OBJECTS: &str = ".git/objects";

    pub fn hash_to_dir_filename(hash: &String) -> (&str, &str) {
        hash.split_at(2)
    }

    pub fn blob_directory(hash: &String) -> String {
        hash.split_at(2).0.to_string()
    }

    pub fn create_object_filename(dir_and_name: &(&str, &str)) -> String {
        GIT_OBJECTS.to_string() + "/" + dir_and_name.0 + "/" + dir_and_name.1
    }

    pub fn object_filename_from_hash(hash: &String) -> String {
        create_object_filename(&hash_to_dir_filename(hash))
    }

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
        use flate2::read::ZlibDecoder;
        use std::io::prelude::*;
        // implements the cat-file pretty-print command of git
        // as input it accepts a sha1 String representing the sha of an object stored in the object directory
        // and it returns the content of that object.
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
            let file_starts = match s.find(" ") {
                Some(index) => index + 1,
                None => 0,
            };
            Ok(s[file_starts..].to_string())
        }

        // This function takes a `sha` of an object and prints the content of the
        // file with the same `sha`.
        pub fn pretty_print(sha_object: &String) -> std::io::Result<()> {
            let file_content = sha_obect_to_string(sha_object)?;
            print!("{}", file_content);
            Ok(())
        }
    }

    pub mod hash {
        use super::*;
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use sha1::Digest;
        use std::fs::File;
        use std::io::prelude::*;

        pub struct Blob {
            content: Vec<u8>,
        }

        impl Blob {
            pub fn from_file(file_name: &str) -> std::io::Result<Self> {
                let mut file = std::fs::File::open(file_name)?;
                let mut content = Vec::new();
                file.read_to_end(&mut content)?;
                Ok(Self { content })
            }

            pub fn from_string(file_content: String) -> Self {
                Self { content: file_content.as_bytes().into() }
            }

            pub fn header(&self) -> String {
                format!("blob {}\0", self.content.len())
            }

            fn hash(&self) -> [u8; 20] {
                let mut hasher = sha1::Sha1::new();
                hasher.update(self.header().as_bytes());
                hasher.update(&self.content);
                hasher.finalize().into()
            }

            // TODO: consider if this function makes sense
            pub fn string_hash(&self) -> String {
                self.hash().iter().fold(String::from(""), |partial_hash: String, element| {
                    partial_hash + &format!("{:02x}", element).to_string()
                }).to_string()
            }

            pub fn compress(&self) -> std::io::Result<Vec<u8>> {
                let mut z = ZlibEncoder::new(Vec::new(), Compression::fast());
                z.write_all(self.header().as_bytes())?;
                z.write_all(&self.content)?;
                z.finish()
            }
        }

        pub fn write_to_database(hash: &String, bytes: &Vec<u8>) -> std::io::Result<()> {
            std::fs::create_dir_all(blob_directory(&hash))?;
            let mut file = File::create(object_filename_from_hash(&hash))?;
            file.write_all(&bytes)
        }

        pub fn write_and_print_hash(file_name: &String) -> std::io::Result<()> {
            let blob = Blob::from_file(file_name)?;
            let blob_hash = blob.string_hash();
            println!("{}", blob_hash);
            write_to_database(&blob_hash, &blob.compress()?)
        }

        #[cfg(test)]
        mod tests {
            use super::*;
            #[test]
            fn testing_hasing_function() {
                if let Ok(my_blob) = Blob::from_file("tests/file_test.txt") {
                    assert_eq!(my_blob.string_hash(), "cd591dba9391e2cdfbae51a51800b9689c7ea360".to_string());
                } else {
                    panic!("Expecting to successfully open a file");
                }
            }

            #[test]
            fn test_hash_of_blob() {
                let my_blob = Blob::from_string("what is up, doc?".to_string());
                assert!(my_blob.header() == "blob 16 ");
                assert_eq!(my_blob.string_hash(), "bd9dbf5aae1a3862dd1526723246b20206e5fc37".to_string());
            }
        }
    }

    pub fn hash_object(args: &[String]) -> Result<(), String> {
        if args[0] == "-w" && args.len() > 1 {
            match hash::write_and_print_hash(&args[1]) {
                Ok(()) => Ok(()),
                Err(e) => Err(format!(
                    "Error: hash-object -w command failed with error: {}",
                    e
                )),
            }
        } else {
            Err("Error: args[0] {}, not a valid hash-object command".to_string())
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
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        if args[1] == "init" {
            plumming::init();
        } else if args[1] == "cat-file" && args.len() > 2 {
            match plumming::cat_file(&args[2..]) {
                Ok(_) => process::exit(0),
                Err(s) => {
                    println!("{}", s);
                    process::exit(-1)
                }
            }
        } else if args[1] == "hash-object" && args.len() > 2 {
            match plumming::hash_object(&args[2..]) {
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

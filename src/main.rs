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
    println!("\t [ls-tre]: List the contents of a tree object.");
    println!("\t\t args: [--name-only] [sha]: List only filenames (instead of the \"long\" output), one per line..");
}

mod plumming {
    use flate2::read::ZlibDecoder;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use sha1::Digest;
    use std::error::Error;
    use std::{fs, fmt};
    use std::io::prelude::*;
    use std::convert::TryInto;
    use std::num::ParseIntError;

    pub const GIT_OBJECTS: &str = ".git/objects";

    // init a git repository by creating the directory structure found in .git
    pub fn init() {
        fs::create_dir(".git").unwrap();
        fs::create_dir(".git/objects").unwrap();
        fs::create_dir(".git/refs").unwrap();
        fs::write(".git/HEAD", "ref: refs/heads/master\n").unwrap();
        println!("Initialized git directory")
    }

    #[derive(Debug, Clone)]
    pub enum EntryType {
        Blob,
        Tree
    }

    #[derive(Debug, Clone)]
    pub struct NotATreeObject;

    pub struct TreeEntry {
        pub mode: String,
        pub entry_type: EntryType,
        pub sha: [u8; 20],
        pub name: String
    }

    pub struct Tree {
        entries: Vec<TreeEntry>
    }

    impl fmt::Display for NotATreeObject {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "content doesn't have the 'tree' header.")
        }
    }

    pub struct Blob {
        pub content: Vec<u8>,
        pub header: Vec<u8>,
        pub hash: [u8; 20],
        pub hash_string: String,
    }


    impl Tree {
        pub fn try_pars(blob: &Blob) -> Result<TreeEntry, NotATreeObject> {
            println!("{}", String::from_utf8_lossy(&blob.header[0..4]));
            if &blob.header[0..4] != "tree".as_bytes() {
                return Err(NotATreeObject);
            } else {
                println!("{}", String::from_utf8_lossy(&blob.content[0..40]));
                Ok (TreeEntry {
                    mode: String::from_utf8_lossy(&blob.content[0..6]).into(),
                    entry_type:  if &blob.content[8..8+4] ==  "blob".as_bytes() { EntryType::Blob }  else { EntryType::Tree},
                    sha: [0; 20],
                    name: "".to_string()
                })
            }
        }
    }

    impl Blob {
        pub fn from_file(file_name: &str) -> std::io::Result<Self> {
            let mut file = std::fs::File::open(file_name)?;
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;
            Ok(Self::from_vec(content))
        }

        fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
            (0..s.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
                .collect()
        }

        pub fn from_sha(sha: &String) -> std::io::Result<Self> {
            let (dir_name, file_name) = sha.split_at(2);
            let full_path = GIT_OBJECTS.to_string() + "/" + dir_name + "/" + file_name;
            let file_content = fs::read(full_path)?;
            let mut z = ZlibDecoder::new(&file_content[..]);
            let byte_sha: [u8; 20] = Blob::decode_hex(sha).unwrap().try_into().unwrap();
            let mut v = Vec::new();
            z.read_to_end(&mut v)?;
            let end_of_header = match v.iter().position(|&x| { x == b' ' }) {
                Some(index) => index + 1,
                None => 0,
            };
            Ok(Self {
                content: v[end_of_header as usize..].into(),
                header: v[0..end_of_header as usize].into(),
                hash: byte_sha,
                hash_string: sha.to_owned()
            })

        }

        pub fn from_vec(content: Vec<u8>) -> Self {
            let header = Self::header(&content);
            let hash = Self::hash(&header, &content);
            let hash_string = Self::string_hash(&hash);
            Self {
                content,
                header,
                hash,
                hash_string,
            }
        }

        pub fn from_string(file_content: String) -> Self {
            Self::from_vec(file_content.as_bytes().into())
        }

        fn header(content: &Vec<u8>) -> Vec<u8> {
            format!("blob {}\0", content.len()).as_bytes().into()
        }

        fn hash(header: &Vec<u8>, content: &Vec<u8>) -> [u8; 20] {
            let mut hasher = sha1::Sha1::new();
            hasher.update(header);
            hasher.update(content);
            hasher.finalize().into()
        }

        // TODO: consider if this function makes sense
        fn string_hash(hash: &[u8]) -> String {
            hash.iter()
                .fold(String::from(""), |partial_hash: String, element| {
                    partial_hash + &format!("{:02x}", element).to_string()
                })
                .to_string()
        }

        pub fn compress(&self) -> std::io::Result<Vec<u8>> {
            let mut z = ZlibEncoder::new(Vec::new(), Compression::fast());
            z.write_all(&self.header)?;
            z.write_all(&self.content)?;
            z.finish()
        }

        pub fn dir(&self) -> String {
            self.hash_string.split_at(2).0.to_string()
        }

        pub fn filename(&self) -> String {
            self.hash_string.split_at(2).1.to_string()
        }
    }


    pub mod cat {
        use super::*;
        use flate2::read::ZlibDecoder;
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
            let blob = Blob::from_sha(sha_object)?;
            println!("{:?}", blob.content[18]);
            println!("{}", String::from_utf8_lossy(&blob.content));
            Ok(())
        }
    }

    pub mod hash {
        use super::*;
        use std::fs::File;

        pub fn write_to_database(blob: &Blob) -> std::io::Result<()> {
            let git_dir = GIT_OBJECTS.to_string() + "/" + &blob.dir();
            std::fs::create_dir_all(&git_dir)?;
            let git_blob_filename = git_dir + "/" + &blob.filename();
            let mut file = File::create(git_blob_filename)?;
            file.write_all(&blob.compress()?)
        }

        pub fn write_and_print_hash(file_name: &String) -> std::io::Result<()> {
            let blob = Blob::from_file(file_name)?;
            let res = write_to_database(&blob);
            println!("{}", blob.hash_string);
            res
        }

        #[cfg(test)]
        mod tests {
            use super::*;
            #[test]
            fn testing_hasing_function() {
                if let Ok(my_blob) = Blob::from_file("tests/file_test.txt") {
                    assert_eq!(
                        my_blob.hash_string,
                        "cd591dba9391e2cdfbae51a51800b9689c7ea360".to_string()
                    );
                } else {
                    panic!("Expecting to successfully open a file");
                }
            }

            #[test]
            fn test_hash_of_blob() {
                let my_blob = Blob::from_string("what is up, doc?".to_string());
                assert!(my_blob.header == "blob 16 ".as_bytes());
                assert_eq!(
                    my_blob.hash_string,
                    "bd9dbf5aae1a3862dd1526723246b20206e5fc37".to_string()
                );
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

    pub fn ls_tree(args: &[String]) -> Result<(), String> {
        if args[0] == "---name-only" && args.len() == 2 {
            Ok(())
        } else {
            Err("Error: args[0] {}, not a valid ls-tree command".to_string())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn tesing_tree_object() {
            let blob = Blob::from_sha(&"20a2555855e4dc0b4b6de8525a35216e3d5985d8".to_string()).unwrap();
            let tree = Tree::try_pars(&blob).unwrap();
            println!("mode: {}, type: {:?}", tree.mode, tree.entry_type);
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
        } else if args[1] == "ls-tree" && args.len() > 2 {
            match plumming::ls_tree(&args[2..]) {
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

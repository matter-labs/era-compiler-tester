//!
//! The tests updater binary.
//!

pub(crate) mod arguments;

use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::Write;

use self::arguments::Arguments;

///
/// Run updating
///
fn main() {
    let arguments = Arguments::new();

    let file = OpenOptions::new()
        .read(true)
        .open(arguments.index.clone())
        .expect("Failed to open file");
    let reader = BufReader::new(file);
    let old_index: solidity_adapter::FSEntity =
        serde_yaml::from_reader(reader).expect("Failed to read index");

    let mut new_index =
        solidity_adapter::FSEntity::index(&arguments.source).expect("Failed to update index");
    let (created, deleted, updated) = old_index
        .update(&mut new_index, arguments.destination.as_path())
        .expect("Failed to update tests");

    println!("{} files created:\n", created.len());
    for file in created {
        println!("{}", file.to_string_lossy());
    }
    println!();

    println!("{} files deleted:\n", deleted.len());
    for file in deleted {
        println!("{}", file.to_string_lossy());
    }
    println!();

    println!("{} files updated:\n", updated.len());
    for (file, conflicts) in updated {
        println!("{} ({})", file.to_string_lossy(), conflicts);
    }
    println!();

    let new_index = serde_yaml::to_string(&new_index).expect("Serialization");
    let mut file_to_write = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(arguments.index)
        .expect("Failed to open file");
    file_to_write
        .write_all(new_index.as_bytes())
        .expect("Failed to write to the output file");

    println!("Test files successfully updated");
}

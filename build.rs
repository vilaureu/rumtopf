use std::{
    env::var_os,
    fs::File,
    fs::{read_dir, DirEntry},
    io::Write,
    path::Path,
};

struct Task<'l> {
    name: &'l str,
    dir: &'l str,
    suffix: &'l str,
    bytes: bool,
}

const TASKS: &[Task] = &[
    Task {
        name: "TEMPLATES",
        dir: "templates",
        suffix: ".html",
        bytes: false,
    },
    Task {
        name: "STATIC",
        dir: "static",
        suffix: "",
        bytes: true,
    },
];
const WRITE_MSG: &str = "failed to write to files.rs";

fn main() {
    let out_dir = var_os("OUT_DIR").expect("OUT_DIR environment variable not found");
    let path = Path::new(&out_dir).join("files.rs");
    let mut output = File::create(path).expect("failed to create files.rs");

    for task in TASKS {
        process_directory(task, &mut output);
    }

    println!("cargo:rerun-if-changed=build.rs");
}

fn process_directory(task: &Task, output: &mut impl Write) {
    writeln!(
        output,
        "pub(crate) const {}: &[(&::std::primitive::str, &{})] = &[",
        task.name,
        if task.bytes {
            "[::std::primitive::u8]"
        } else {
            "::std::primitive::str"
        }
    )
    .expect(WRITE_MSG);

    let dir_path = Path::new("src").join(task.dir);
    let iterator = read_dir(&dir_path).expect("failed to read directory");
    for input in iterator {
        process_file(task, &input.expect("failed to iterate directory"), output);
    }

    writeln!(output, "];").expect(WRITE_MSG);
    println!(
        "cargo:rerun-if-changed={}",
        dir_path.to_str().expect("path is not unicode")
    );
}

fn process_file(task: &Task, input: &DirEntry, output: &mut impl Write) {
    if !input
        .file_type()
        .expect("failed to query file type")
        .is_file()
    {
        return;
    }

    let file_name_os = input.file_name();
    let file_name = file_name_os.to_str().expect("file name is not unicode");
    if file_name.starts_with('.') {
        return;
    }
    let Some(file_name) = file_name.strip_suffix(task.suffix) else {
        return;
    };

    let path_os = input.path();
    let path = path_os.to_str().expect("path is not unicode");
    writeln!(
        output,
        r#"("{file_name}", ::std::include_{}!(::std::concat!(::std::env!("CARGO_MANIFEST_DIR"), "/{}"))),"#,
        if task.bytes { "bytes" } else { "str" },
        path
    )
    .expect(WRITE_MSG);
}

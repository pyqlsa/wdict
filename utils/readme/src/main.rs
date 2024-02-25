use std::io::{BufRead, Write};
use std::path::Path;
use std::process::Command;
use std::{env, fs};

fn main() {
    let readme_file = "README.md";
    let proj_root = Path::new("..").join("..");
    let start_tag = "<!-- readme-help -->\n";
    let end_tag = "<!-- readme-help end -->\n";

    assert!(env::set_current_dir(&proj_root).is_ok());

    let readme = fs::read_to_string(readme_file).unwrap();
    let help_start = readme.find(start_tag).unwrap();
    let help_end = readme.find(end_tag).unwrap();

    println!(
        "Found help section in readme starting at {}, ending at {}",
        help_start, help_end
    );
    assert!(help_start < help_end, "start >= end, not continuing...");

    println!("Getting latest help content...");
    let help_content = get_help();

    let readme_begin = &readme[..help_start];
    let readme_end = &readme[(help_end + end_tag.len())..];
    let fin_readme = format!(
        "{}{}```bash\n{}\n```\n{}{}",
        readme_begin, start_tag, help_content, end_tag, readme_end
    );

    println!("Writing out final readme...");
    let mut file = fs::File::create(readme_file).expect("Error creating final readme file");
    file.write_all(fin_readme.as_bytes())
        .expect("Failed writing final readme file");

    println!("Final readme written");
}

fn get_help() -> String {
    let cmd = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("--help")
        .output()
        .expect("failed to execute process");

    // --- This method leaves lines that consist of just whitespace...
    // let output = cmd.stdout;
    // match String::from_utf8(output) {
    //     Ok(v) => v,
    //     Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
    // };
    // --- ...so we do this instead to truncate lines w/ just whitepsace

    let mut fin = "".to_owned();
    let out = cmd.stdout.as_slice();
    for line in out.lines() {
        match line {
            Ok(l) => {
                if String::from(l.as_str()).trim().len() < 1 {
                    fin.push_str("\n");
                } else {
                    fin.push_str(format!("{}\n", l).as_str());
                }
            }
            Err(e) => panic!("derp; failed reading output: {}", e),
        }
    }
    fin
}

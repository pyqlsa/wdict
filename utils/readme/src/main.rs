use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::{env, fs, str};

fn main() {
    let readme_file = "README.md";
    let proj_root = Path::new("..").join("..");
    let start_tag = "<!-- readme-help -->\n";
    let end_tag = "<!-- readme-help end -->\n";

    assert!(env::set_current_dir(&proj_root).is_ok());

    let readme = read_file_string(readme_file).unwrap();
    let help_start = readme.find(start_tag).unwrap();
    let help_end = readme.find(end_tag).unwrap();

    println!(
        "Found help section in readme starting at {}, ending at {}",
        help_start, help_end
    );

    if help_start > help_end {
        eprintln!("start greater than end, not continuing...");
        return;
    }

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

fn read_file_string(filepath: &str) -> Result<String, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(filepath)?;
    Ok(data)
}

fn get_help() -> String {
    let cmd = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("--help")
        .output()
        .expect("failed to execute process");

    let output = cmd.stdout;
    match String::from_utf8(output) {
        Ok(v) => v,
        Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
    }
}

use clap::{ArgAction, Parser};
use std::fs::{self, DirEntry, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

mod tree;

#[derive(Parser)]
#[command(author, version, about = "Concatenates files in a directory", long_about = None)]
struct Cli {
    /// Sets the input directory to use
    #[arg(short, long, value_name = "DIR")]
    directory: PathBuf,

    /// Sets the output file
    #[arg(short, long, value_name = "FILE")]
    output: PathBuf,

    /// File extensions to include (whitelist), comma-separated
    #[arg(long, use_value_delimiter = true)]
    include_extensions: Option<Vec<String>>,

    /// File extensions to exclude (blacklist), comma-separated
    #[arg(long, use_value_delimiter = true)]
    exclude_extensions: Option<Vec<String>>,

    /// Maximum depth for recursive search
    #[arg(long, default_value_t = usize::MAX)]
    max_depth: usize,

    /// Flag to write filenames as comments
    #[arg(long, action = ArgAction::SetTrue, default_value_t = true)]
    write_filenames: bool,

    /// Flag to write directory tree at the top of the output file
    #[arg(long, action = ArgAction::SetTrue, default_value_t = true)]
    write_tree: bool,

    /// Comment style to use for filenames (default: //)
    #[arg(long, default_value = "//")]
    comment_style: String,

    /// Buffer size for writing (in bytes)
    #[arg(long, default_value_t = 8192)]
    buffer_size: usize,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    concatenate_files(&cli)
}

fn concatenate_files(cli: &Cli) -> io::Result<()> {
    let file = File::create(&cli.output)?;
    let mut writer = BufWriter::with_capacity(cli.buffer_size, file);
    let directory = &cli.directory;
    let output_path = fs::canonicalize(&cli.output)?;

    if cli.write_tree {
        writeln!(writer, "{}", tree::tree(directory)?.to_string())?;
    }

    visit_dirs(
        directory,
        cli,
        &mut |entry| {
            let path = entry.path();
            if !path.is_file() {
                return Ok(());
            }
            let canonical_path = fs::canonicalize(&path)?;
            if canonical_path == output_path {
                return Ok(());
            }

            if should_process_file(&path, cli) {
                if cli.write_filenames {
                    writeln!(writer, "{} {}", cli.comment_style, path.display())?;
                }
                let contents = fs::read(&path)?;
                writer.write_all(&contents)?;
                writeln!(writer)?;
            }
            Ok(())
        },
        0,
    )?;

    writer.flush()?;
    Ok(())
}

fn should_process_file(path: &Path, cli: &Cli) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|ext| {
            let include = cli
                .include_extensions
                .as_ref()
                .map_or(true, |v| v.contains(&ext.to_string()));
            let exclude = cli
                .exclude_extensions
                .as_ref()
                .map_or(false, |v| v.contains(&ext.to_string()));
            include && !exclude
        })
        .unwrap_or(false)
}

fn visit_dirs<F>(dir: &Path, cli: &Cli, cb: &mut F, depth: usize) -> io::Result<()>
where
    F: FnMut(&DirEntry) -> io::Result<()>,
{
    if depth > cli.max_depth {
        return Ok(());
    }

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cli, cb, depth + 1)?;
            } else {
                cb(&entry)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Read;
    use tempfile::TempDir;

    fn create_test_directory() -> TempDir {
        let dir = TempDir::new().unwrap();
        let path = dir.path();

        fs::write(path.join("file1.txt"), "Content of file1").unwrap();
        fs::write(path.join("file2.rs"), "Content of file2").unwrap();
        fs::create_dir(path.join("subdir")).unwrap();
        fs::write(path.join("subdir").join("file3.txt"), "Content of file3").unwrap();

        dir
    }

    #[test]
    fn test_concatenate_files() {
        let temp_dir = create_test_directory();
        let output_file = temp_dir.path().join("output.txt");

        let cli = Cli {
            directory: temp_dir.path().to_path_buf(),
            output: output_file.clone(),
            include_extensions: None,
            exclude_extensions: None,
            max_depth: usize::MAX,
            write_filenames: true,
            write_tree: false,
            comment_style: "//".to_string(),
            buffer_size: 8192,
        };

        concatenate_files(&cli).unwrap();

        let mut output_content = String::new();
        File::open(output_file)
            .unwrap()
            .read_to_string(&mut output_content)
            .unwrap();

        assert!(output_content.contains("Content of file1"));
        assert!(output_content.contains("Content of file2"));
        assert!(output_content.contains("Content of file3"));
        assert!(output_content.contains("// "));
    }

    #[test]
    fn test_include_extensions() {
        let temp_dir = create_test_directory();
        let output_file = temp_dir.path().join("output.txt");

        let cli = Cli {
            directory: temp_dir.path().to_path_buf(),
            output: output_file.clone(),
            include_extensions: Some(vec!["txt".to_string()]),
            exclude_extensions: None,
            max_depth: usize::MAX,
            write_filenames: false,
            write_tree: false,
            comment_style: "//".to_string(),
            buffer_size: 8192,
        };

        concatenate_files(&cli).unwrap();

        let mut output_content = String::new();
        File::open(output_file)
            .unwrap()
            .read_to_string(&mut output_content)
            .unwrap();

        assert!(output_content.contains("Content of file1"));
        assert!(!output_content.contains("Content of file2"));
        assert!(output_content.contains("Content of file3"));
    }

    #[test]
    fn test_exclude_extensions() {
        let temp_dir = create_test_directory();
        let output_file = temp_dir.path().join("output.txt");

        let cli = Cli {
            directory: temp_dir.path().to_path_buf(),
            output: output_file.clone(),
            include_extensions: None,
            exclude_extensions: Some(vec!["rs".to_string()]),
            max_depth: usize::MAX,
            write_filenames: false,
            write_tree: false,
            comment_style: "//".to_string(),
            buffer_size: 8192,
        };

        concatenate_files(&cli).unwrap();

        let mut output_content = String::new();
        File::open(output_file)
            .unwrap()
            .read_to_string(&mut output_content)
            .unwrap();

        assert!(output_content.contains("Content of file1"));
        assert!(!output_content.contains("Content of file2"));
        assert!(output_content.contains("Content of file3"));
    }

    #[test]
    fn test_max_depth() {
        let temp_dir = create_test_directory();
        let output_file = temp_dir.path().join("output.txt");

        let cli = Cli {
            directory: temp_dir.path().to_path_buf(),
            output: output_file.clone(),
            include_extensions: None,
            exclude_extensions: None,
            max_depth: 0,
            write_filenames: false,
            write_tree: false,
            comment_style: "//".to_string(),
            buffer_size: 8192,
        };

        concatenate_files(&cli).unwrap();

        let mut output_content = String::new();
        File::open(output_file)
            .unwrap()
            .read_to_string(&mut output_content)
            .unwrap();

        assert!(output_content.contains("Content of file1"));
        assert!(output_content.contains("Content of file2"));
        assert!(!output_content.contains("Content of file3"));
    }

    #[test]
    fn test_custom_comment_style() {
        let temp_dir = create_test_directory();
        let output_file = temp_dir.path().join("output.txt");

        let cli = Cli {
            directory: temp_dir.path().to_path_buf(),
            output: output_file.clone(),
            include_extensions: None,
            exclude_extensions: None,
            max_depth: usize::MAX,
            write_filenames: true,
            write_tree: false,
            comment_style: "#".to_string(),
            buffer_size: 8192,
        };

        concatenate_files(&cli).unwrap();

        let mut output_content = String::new();
        File::open(output_file)
            .unwrap()
            .read_to_string(&mut output_content)
            .unwrap();

        assert!(output_content.contains("# "));
        assert!(!output_content.contains("// "));
    }
}

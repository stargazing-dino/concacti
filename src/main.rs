use clap::{ArgAction, Parser};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::fs::{self, DirEntry, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

mod tree;

#[derive(Parser)]
#[command(
    author, 
    version, 
    about = "Concatenates files in a directory", 
    long_about = None,
    after_help = "EXAMPLES:
    # Concatenate all .ts files, excluding those in node_modules
    file_concatenator -d ./src -o output.txt -p '**/*.ts' -p '!**/node_modules/**'

    # Concatenate all files, limit depth to 2, and write tree
    file_concatenator -d ./project -o output.txt --max-depth 2 --write-tree

    # Use custom comment style and buffer size
    file_concatenator -d ./docs -o output.md -p '**/*.md' --comment-style '<!--' --buffer-size 16384
"
)]
struct Cli {
    /// Sets the input directory to use
    #[arg(short, long, value_name = "DIR")]
    directory: PathBuf,

    /// Sets the output file
    #[arg(short, long, value_name = "FILE")]
    output: PathBuf,

    /// File patterns to include or exclude (use ! for exclusion), comma-separated
    #[arg(short, long, use_value_delimiter = true)]
    patterns: Vec<String>,

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

struct FileFilter {
    include: GlobSet,
    exclude: GlobSet,
    include_all: bool,
}

impl FileFilter {
    fn new(patterns: &[String]) -> Result<Self, globset::Error> {
        let mut include_builder = GlobSetBuilder::new();
        let mut exclude_builder = GlobSetBuilder::new();
        let mut include_all = true;

        for pattern in patterns {
            if let Some(pattern) = pattern.strip_prefix('!') {
                exclude_builder.add(Glob::new(pattern)?);
                include_all = false;
            } else {
                include_builder.add(Glob::new(pattern)?);
                include_all = false;
            }
        }

        if include_all {
            include_builder.add(Glob::new("**/*")?);
        }

        Ok(FileFilter {
            include: include_builder.build()?,
            exclude: exclude_builder.build()?,
            include_all,
        })
    }

    fn should_process(&self, path: &Path) -> bool {
        (self.include_all || self.include.is_match(path)) && !self.exclude.is_match(path)
    }
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

    let file_filter = FileFilter::new(&cli.patterns)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    if cli.write_tree {
        writeln!(writer, "{}", tree::tree(directory)?.to_string())?;
    }

    visit_dirs(
        directory,
        cli,
        &file_filter,
        &mut |entry| {
            let path = entry.path();
            if !path.is_file() {
                return Ok(());
            }
            let canonical_path = fs::canonicalize(&path)?;
            if canonical_path == output_path {
                return Ok(());
            }

            if file_filter.should_process(&path) {
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

fn visit_dirs<F>(
    dir: &Path,
    cli: &Cli,
    file_filter: &FileFilter,
    cb: &mut F,
    depth: usize,
) -> io::Result<()>
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
                visit_dirs(&path, cli, file_filter, cb, depth + 1)?;
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
        fs::write(path.join("file2.ts"), "Content of file2").unwrap();
        fs::create_dir(path.join("subdir")).unwrap();
        fs::write(path.join("subdir").join("file3.ts"), "Content of file3").unwrap();
        fs::create_dir(path.join("node_modules")).unwrap();
        fs::write(
            path.join("node_modules").join("file4.ts"),
            "Content of file4",
        )
        .unwrap();

        dir
    }

    #[test]
    fn test_wildcard_include() {
        let temp_dir = create_test_directory();
        let output_file = temp_dir.path().join("output.txt");

        let cli = Cli {
            directory: temp_dir.path().to_path_buf(),
            output: output_file.clone(),
            patterns: vec!["**/*.ts".to_string()],
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

        assert!(!output_content.contains("Content of file1"));
        assert!(output_content.contains("Content of file2"));
        assert!(output_content.contains("Content of file3"));
        assert!(output_content.contains("Content of file4"));
    }

    #[test]
    fn test_wildcard_exclude() {
        let temp_dir = create_test_directory();
        let output_file = temp_dir.path().join("output.txt");

        let cli = Cli {
            directory: temp_dir.path().to_path_buf(),
            output: output_file.clone(),
            patterns: vec!["**/*.ts".to_string(), "!**/node_modules/**".to_string()],
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

        assert!(!output_content.contains("Content of file1"));
        assert!(output_content.contains("Content of file2"));
        assert!(output_content.contains("Content of file3"));
        assert!(!output_content.contains("Content of file4"));
    }

    #[test]
    fn test_multiple_patterns() {
        let temp_dir = create_test_directory();
        let output_file = temp_dir.path().join("output.txt");

        let cli = Cli {
            directory: temp_dir.path().to_path_buf(),
            output: output_file.clone(),
            patterns: vec![
                "**/*.ts".to_string(),
                "**/*.txt".to_string(),
                "!**/node_modules/**".to_string(),
            ],
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
        assert!(output_content.contains("Content of file2"));
        assert!(output_content.contains("Content of file3"));
        assert!(!output_content.contains("Content of file4"));
    }

    #[test]
    fn test_no_patterns() {
        let temp_dir = create_test_directory();
        let output_file = temp_dir.path().join("output.txt");

        let cli = Cli {
            directory: temp_dir.path().to_path_buf(),
            output: output_file.clone(),
            patterns: vec![],
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
        assert!(output_content.contains("Content of file2"));
        assert!(output_content.contains("Content of file3"));
        assert!(output_content.contains("Content of file4"));
    }

    #[test]
    fn test_max_depth() {
        let temp_dir = create_test_directory();
        let output_file = temp_dir.path().join("output.txt");

        let cli = Cli {
            directory: temp_dir.path().to_path_buf(),
            output: output_file.clone(),
            patterns: vec!["**/*.ts".to_string()],
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

        assert!(output_content.contains("Content of file2"));
        assert!(!output_content.contains("Content of file3"));
        assert!(!output_content.contains("Content of file4"));
    }

    #[test]
    fn test_comment_style() {
        let temp_dir = create_test_directory();
        let output_file = temp_dir.path().join("output.txt");

        let cli = Cli {
            directory: temp_dir.path().to_path_buf(),
            output: output_file.clone(),
            patterns: vec!["**/*.ts".to_string()],
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

    #[test]
    fn test_write_filenames() {
        let temp_dir = create_test_directory();
        let output_file = temp_dir.path().join("output.txt");

        let cli = Cli {
            directory: temp_dir.path().to_path_buf(),
            output: output_file.clone(),
            patterns: vec!["**/*.ts".to_string()],
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

        assert!(output_content.contains("// "));
        assert!(output_content.contains("file2.ts"));
        assert!(output_content.contains("file3.ts"));
    }

    #[test]
    fn test_write_tree() {
        let temp_dir = create_test_directory();
        let output_file = temp_dir.path().join("output.txt");

        let cli = Cli {
            directory: temp_dir.path().to_path_buf(),
            output: output_file.clone(),
            patterns: vec!["**/*.ts".to_string()],
            max_depth: usize::MAX,
            write_filenames: false,
            write_tree: true,
            comment_style: "//".to_string(),
            buffer_size: 8192,
        };

        concatenate_files(&cli).unwrap();

        let mut output_content = String::new();
        File::open(output_file)
            .unwrap()
            .read_to_string(&mut output_content)
            .unwrap();

        assert!(output_content.contains("subdir"));
        assert!(output_content.contains("node_modules"));
        assert!(output_content.contains("file2.ts"));
        assert!(output_content.contains("file3.ts"));
        assert!(output_content.contains("file4.ts"));
    }

    #[test]
    fn test_buffer_size() {
        let temp_dir = create_test_directory();
        let output_file = temp_dir.path().join("output.txt");

        let cli = Cli {
            directory: temp_dir.path().to_path_buf(),
            output: output_file.clone(),
            patterns: vec!["**/*.ts".to_string()],
            max_depth: usize::MAX,
            write_filenames: false,
            write_tree: false,
            comment_style: "//".to_string(),
            buffer_size: 1, // Minimum buffer size to test buffering
        };

        concatenate_files(&cli).unwrap();

        let mut output_content = String::new();
        File::open(output_file)
            .unwrap()
            .read_to_string(&mut output_content)
            .unwrap();

        assert!(output_content.contains("Content of file2"));
        assert!(output_content.contains("Content of file3"));
        assert!(output_content.contains("Content of file4"));
    }
}

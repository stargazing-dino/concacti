use clap::{ArgAction, Parser};
use std::fs::{self, DirEntry};
use std::io::{self, Write};
use std::path::Path;
use std::path::PathBuf;

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
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    concatenate_files(&cli)
}

// if cli.write_tree {
//     writeln!(output, "{}", tree(directory)?.to_string())?;
// }
fn concatenate_files(cli: &Cli) -> io::Result<()> {
    let mut output = fs::File::create(&cli.output)?;
    let directory = &cli.directory;
    let output_path = fs::canonicalize(&cli.output)?;

    visit_dirs(
        directory,
        &cli,
        &mut |entry| {
            let path = entry.path();
            if !path.is_file() {
                return Ok(());
            }
            let canonical_path = fs::canonicalize(&path)?;
            if canonical_path == output_path {
                return Ok(());
            }

            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                let include = cli
                    .include_extensions
                    .as_ref()
                    .map_or(true, |v| v.contains(&ext.to_string()));
                let exclude = cli
                    .exclude_extensions
                    .as_ref()
                    .map_or(false, |v| v.contains(&ext.to_string()));

                if include && !exclude {
                    if cli.write_filenames {
                        writeln!(output, "// {}", path.display())?;
                    }
                    let contents = fs::read(&path)?;
                    output.write_all(&contents)?;

                    writeln!(output)?;
                }
            }
            Ok(())
        },
        0,
    )
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

# Concacti

Concacti is a command-line tool for concatenating files in a directory based on specified patterns. It offers flexible file selection, custom output formatting, and directory tree visualization.

## Features

- Concatenate files based on glob patterns
- Exclude files or directories using negative patterns
- Limit search depth
- Write filenames as comments in the output
- Generate and include a directory tree in the output
- Customize comment style for filenames
- Adjustable buffer size for optimized writing

## Installation

To install Concacti, you need to have Rust and Cargo installed on your system. Then, you can build and install the project using:

```
cargo install --path .
```

## Usage

```
concacti [OPTIONS] --directory <DIR> --output <FILE>
```

## Examples

1. Concatenate all .ts files, excluding those in node_modules:
   ```
   concacti -d ./src -o output.txt -p '**/*.ts' -p '!**/node_modules/**'
   ```

2. Concatenate all files, limit depth to 2, and write tree:
   ```
   concacti -d ./project -o output.txt --max-depth 2 --write-tree
   ```

3. Use custom comment style and buffer size:
   ```
   concacti -d ./docs -o output.md -p '**/*.md' --comment-style '<!--' --buffer-size 16384
   ```
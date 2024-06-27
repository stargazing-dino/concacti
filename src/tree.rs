use std::fs::{self};
use std::io::{self};
use std::path::Path;
use termtree::Tree;

fn label<P: AsRef<Path>>(p: P) -> String {
    p.as_ref().file_name().unwrap().to_str().unwrap().to_owned()
}

pub fn tree<P: AsRef<Path>>(p: P) -> io::Result<Tree<String>> {
    let result = fs::read_dir(&p)?.filter_map(|e| e.ok()).fold(
        Tree::new(label(p.as_ref().canonicalize()?)),
        |mut root, entry| {
            let dir = entry.metadata().unwrap();
            if dir.is_dir() {
                root.push(tree(entry.path()).unwrap());
            } else {
                root.push(Tree::new(label(entry.path())));
            }
            root
        },
    );
    Ok(result)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn create_test_directory() -> TempDir {
        let dir = TempDir::new().unwrap();
        let path = dir.path();

        fs::create_dir(path.join("dir1")).unwrap();
        fs::create_dir(path.join("dir2")).unwrap();
        fs::create_dir(path.join("dir1").join("subdir1")).unwrap();
        fs::write(path.join("file1.txt"), "Content of file1").unwrap();
        fs::write(path.join("dir1").join("file2.txt"), "Content of file2").unwrap();
        fs::write(path.join("dir2").join("file3.txt"), "Content of file3").unwrap();
        fs::write(
            path.join("dir1").join("subdir1").join("file4.txt"),
            "Content of file4",
        )
        .unwrap();

        dir
    }

    #[test]
    fn test_label() {
        let path = Path::new("/tmp/test/file.txt");
        assert_eq!(label(path), "file.txt");

        let path = Path::new("/tmp/test/");
        assert_eq!(label(path), "test");
    }

    #[test]
    fn test_tree_root() {
        let temp_dir = create_test_directory();
        let tree_result = tree(temp_dir.path()).unwrap();

        assert_eq!(
            tree_result.root,
            temp_dir
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned()
        );
    }

    #[test]
    fn test_tree_structure() {
        let temp_dir = create_test_directory();
        let tree_result = tree(temp_dir.path()).unwrap();

        let tree_string = tree_result.to_string();
        println!("Tree structure:\n{}", tree_string);

        // Check if all directories and files are present in the tree
        assert!(tree_string.contains("dir1"));
        assert!(tree_string.contains("dir2"));
        assert!(tree_string.contains("subdir1"));
        assert!(tree_string.contains("file1.txt"));
        assert!(tree_string.contains("file2.txt"));
        assert!(tree_string.contains("file3.txt"));
        assert!(tree_string.contains("file4.txt"));
    }

    #[test]
    fn test_tree_depth() {
        let temp_dir = create_test_directory();
        let tree_result = tree(temp_dir.path()).unwrap();

        let tree_string = tree_result.to_string();
        let lines: Vec<&str> = tree_string.lines().collect();

        // Check the depth of the tree
        assert!(lines.iter().any(|&line| line.starts_with("└── dir1")));
        assert!(lines
            .iter()
            .any(|&line| line.starts_with("    └── subdir1")));
        assert!(lines
            .iter()
            .any(|&line| line.starts_with("        └── file4.txt")));
    }

    #[test]
    fn test_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let tree_result = tree(temp_dir.path()).unwrap();

        let tree_string = tree_result.to_string();
        assert_eq!(
            tree_string.lines().count(),
            1,
            "Empty directory should only have the root node"
        );
    }

    #[test]
    fn test_nonexistent_directory() {
        let result = tree(Path::new("/nonexistent/directory"));
        assert!(
            result.is_err(),
            "Attempting to create a tree for a nonexistent directory should return an error"
        );
    }
}

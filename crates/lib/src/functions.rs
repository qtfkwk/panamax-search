use {
    anyhow::{anyhow, Result},
    log::error,
    std::path::Path,
    walkdir::DirEntry,
};

pub fn path_parent(path: &Path, levels: usize) -> &Path {
    let mut r = path;
    for _ in 0..levels {
        r = r.parent().unwrap();
    }
    r
}

pub fn filter_entries(entry: &DirEntry) -> bool {
    if entry.file_type().is_dir() {
        !entry.file_name().to_str().unwrap().starts_with('.')
    } else {
        entry.depth() > 1
    }
}

pub fn ensure_directory(directory: &Path) -> Result<()> {
    if !directory.exists() {
        error!("Directory does not exist {directory:?}");
        return Err(anyhow!("Directory does not exist {directory:?}"));
    }

    if !directory.is_dir() {
        error!("Directory is not a directory {directory:?}");
        return Err(anyhow!("Directory is not a directory {directory:?}"));
    }

    Ok(())
}

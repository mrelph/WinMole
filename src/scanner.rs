//! Shared bounded filesystem traversal and accounting.

use serde::Serialize;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use walkdir::WalkDir;

#[derive(Debug, Clone, Default, Serialize)]
pub struct ScanStats {
    pub files: u64,
    pub directories: u64,
    pub bytes: u64,
    pub skipped: u64,
    pub cancelled: bool,
    pub elapsed_ms: u128,
}

pub fn scan_files(
    root: &Path,
    cancellation: Option<&AtomicBool>,
    mut visitor: impl FnMut(&Path, &std::fs::Metadata),
) -> ScanStats {
    let started = Instant::now();
    let mut stats = ScanStats::default();

    for entry in WalkDir::new(root) {
        if cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            stats.cancelled = true;
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                stats.skipped += 1;
                continue;
            }
        };
        if entry.file_type().is_dir() {
            stats.directories += 1;
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => {
                stats.skipped += 1;
                continue;
            }
        };
        stats.files += 1;
        stats.bytes = stats.bytes.saturating_add(metadata.len());
        visitor(entry.path(), &metadata);
    }
    stats.elapsed_ms = started.elapsed().as_millis();
    stats
}

pub fn retain_top_n<T: Ord>(heap: &mut BinaryHeap<Reverse<T>>, item: T, limit: usize) {
    if limit == 0 {
        return;
    }
    heap.push(Reverse(item));
    if heap.len() > limit {
        heap.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_heap_keeps_largest_items() {
        let mut heap = BinaryHeap::new();
        for value in [5, 1, 9, 2, 8] {
            retain_top_n(&mut heap, value, 3);
        }
        let mut values: Vec<_> = heap.into_iter().map(|Reverse(value)| value).collect();
        values.sort_unstable();
        assert_eq!(values, vec![5, 8, 9]);
    }

    #[test]
    fn scan_counts_files_and_bytes() {
        let directory = std::env::temp_dir().join(format!("winmole-scan-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("a"), b"123").unwrap();
        std::fs::write(directory.join("b"), b"4567").unwrap();

        let stats = scan_files(&directory, None, |_, _| {});
        assert_eq!(stats.files, 2);
        assert_eq!(stats.bytes, 7);
        assert!(!stats.cancelled);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

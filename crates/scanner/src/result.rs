use std::path::{Path, PathBuf};
use std::time::Duration;

/// Index of a node inside a [`ScanResult`]. The root is always `0`.
pub type NodeId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Folder,
    File,
}

/// Whether the Scan ran to the end or was cancelled midway.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanState {
    Completed,
    /// The result only covers what was read before cancellation.
    Cancelled,
}

/// One file or folder of the result tree.
#[derive(Debug, Clone)]
pub struct Node {
    pub(crate) name: Box<str>,
    pub(crate) kind: NodeKind,
    pub(crate) size: u64,
    pub(crate) files: u64,
    pub(crate) folders: u64,
    pub(crate) modified: Option<i64>,
    pub(crate) parent: Option<NodeId>,
    pub(crate) first_child: NodeId,
    pub(crate) child_count: u32,
}

impl Node {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> NodeKind {
        self.kind
    }

    /// Logical size in bytes; for a folder, the sum of everything below it.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Number of files below this folder, at any depth (0 for a file).
    pub fn files(&self) -> u64 {
        self.files
    }

    /// Number of folders below this folder, at any depth (0 for a file).
    pub fn folders(&self) -> u64 {
        self.folders
    }

    /// Last modification time, in milliseconds since the Unix epoch.
    pub fn modified(&self) -> Option<i64> {
        self.modified
    }

    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    /// Number of direct children (subfolders and loose files).
    pub fn child_count(&self) -> usize {
        self.child_count as usize
    }
}

/// The indexed result of a Scan, with queries on the tree.
#[derive(Debug)]
pub struct ScanResult {
    pub(crate) target: PathBuf,
    pub(crate) nodes: Vec<Node>,
    pub(crate) top_files: Vec<NodeId>,
    pub(crate) unreadable: Vec<PathBuf>,
    pub(crate) duration: Duration,
    pub(crate) state: ScanState,
}

impl ScanResult {
    pub fn target(&self) -> &Path {
        &self.target
    }

    pub fn root(&self) -> NodeId {
        0
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id as usize)
    }

    /// Direct children of `id`, sorted by descending size (ties by name),
    /// skipping `offset` and returning at most `limit` ids.
    pub fn children(&self, id: NodeId, offset: usize, limit: usize) -> Vec<NodeId> {
        let Some(node) = self.node(id) else {
            return Vec::new();
        };
        let count = node.child_count as usize;
        let start = offset.min(count);
        let end = start.saturating_add(limit).min(count);
        (start..end)
            .map(|i| node.first_child + i as NodeId)
            .collect()
    }

    /// Up to `n` Large files of the whole Target, by descending size. Ties
    /// are broken by position in the tree, so the order is deterministic.
    /// At most [`ScanOptions::top_n`](crate::ScanOptions::top_n) are kept.
    pub fn top_files(&self, n: usize) -> &[NodeId] {
        &self.top_files[..n.min(self.top_files.len())]
    }

    /// Full path of a node.
    pub fn path(&self, id: NodeId) -> Option<PathBuf> {
        let mut names = Vec::new();
        let mut current = self.node(id)?;
        while let Some(parent) = current.parent {
            names.push(&*current.name);
            current = &self.nodes[parent as usize];
        }
        let mut path = self.target.clone();
        path.extend(names.iter().rev());
        Some(path)
    }

    pub fn total_size(&self) -> u64 {
        self.nodes[0].size
    }

    pub fn total_files(&self) -> u64 {
        self.nodes[0].files
    }

    pub fn total_folders(&self) -> u64 {
        self.nodes[0].folders
    }

    /// Folders that could not be read; their contents are missing from the totals.
    pub fn unreadable(&self) -> &[PathBuf] {
        &self.unreadable
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn state(&self) -> ScanState {
        self.state
    }
}

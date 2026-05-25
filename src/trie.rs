use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cmp::min;

use crate::keccak::keccak256;
use crate::nibble::{hp_decode, hp_encode, NibbleBuf, MAX_NIBBLES};
use crate::rlp::{decode_strict, encode_list, encode_str, RlpItem};

// ============================================================
// Constants
// ============================================================

/// `keccak256(rlp(b""))` = root hash of an empty trie.
pub const EMPTY_ROOT_HASH: [u8; 32] = [
    0x56, 0xe8, 0x1f, 0x17, 0x1b, 0xcc, 0x55, 0xa6, 0xff, 0x83, 0x45, 0xe6, 0x92, 0x0c, 0xf8, 0x6e,
    0x5b, 0x48, 0xe0, 0x1b, 0x99, 0x6c, 0xad, 0xc0, 0x01, 0x62, 0x2f, 0xb5, 0xe3, 0x63, 0xb4, 0x21,
];

// ============================================================
// Error type
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Database,
    MissingNode,
    DecodeFailed,
}

// ============================================================
// Node types
// ============================================================

/// A decoded trie node.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Node {
    Empty,
    Leaf {
        path: NibbleBuf,
        value: Vec<u8>,
    },
    Extension {
        path: NibbleBuf,
        child: Box<NodeRef>,
    },
    Branch {
        children: [Option<Box<NodeRef>>; 16],
        value: Option<Vec<u8>>,
    },
}

impl Default for Node {
    fn default() -> Self {
        Self::Empty
    }
}

/// A reference to a trie node — either empty, a hash pointer, or inlined.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum NodeRef {
    Empty,
    Hash([u8; 32]),
    Inline(Box<Node>),
}

fn empty_children() -> [Option<Box<NodeRef>>; 16] {
    [
        None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
        None,
    ]
}

// ============================================================
// Trie
// ============================================================

/// A Merkle Patricia Trie backed by a `Database`.
#[derive(Clone, Debug)]
pub struct Trie {
    root: Node,
}

impl Trie {
    #[must_use]
    pub fn new() -> Self {
        Self { root: Node::Empty }
    }

    /// Decode a trie from a persisted root hash.
    pub fn from_root(db: &dyn super::db::Database, root_hash: &[u8; 32]) -> Result<Self, Error> {
        if *root_hash == EMPTY_ROOT_HASH {
            return Ok(Self::new());
        }
        let data = db
            .get(root_hash)
            .map_err(|_| Error::Database)?
            .ok_or(Error::MissingNode)?;
        let root = decode_node(&data)?;
        Ok(Self { root })
    }

    /// Look up a value by its raw key.
    pub fn get(&self, db: &dyn super::db::Database, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        let nibbles = NibbleBuf::from_key(key);
        get_internal(db, &self.root, nibbles.as_nibbles())
    }

    /// Insert a key-value pair into the trie.
    ///
    /// On error (e.g. database failure), the trie root is reset to `Empty`.
    /// The caller should recover from a persisted root hash.
    pub fn insert(
        &mut self,
        db: &mut dyn super::db::Database,
        key: &[u8],
        value: Vec<u8>,
    ) -> Result<(), Error> {
        let nibbles = NibbleBuf::from_key(key);
        let old = core::mem::take(&mut self.root);
        match insert_internal(db, old, nibbles.as_nibbles(), value) {
            Ok(new_root) => {
                self.root = new_root;
                Ok(())
            }
            Err(e) => {
                self.root = Node::Empty;
                Err(e)
            }
        }
    }

    /// Remove a key from the trie.
    ///
    /// On error (e.g. database failure), the trie root is reset to `Empty`.
    /// The caller should recover from a persisted root hash.
    pub fn remove(&mut self, db: &mut dyn super::db::Database, key: &[u8]) -> Result<(), Error> {
        let nibbles = NibbleBuf::from_key(key);
        let old = core::mem::take(&mut self.root);
        match remove_internal(db, old, nibbles.as_nibbles()) {
            Ok(new_root) => {
                self.root = new_root;
                Ok(())
            }
            Err(e) => {
                self.root = Node::Empty;
                Err(e)
            }
        }
    }

    /// Compute the root hash, writing all dirty nodes to the database.
    pub fn root_hash(&self, db: &mut dyn super::db::Database) -> Result<[u8; 32], Error> {
        if self.root == Node::Empty {
            return Ok(EMPTY_ROOT_HASH);
        }
        let rlp = rlp_encode_node(&self.root);
        let hash = keccak256(&rlp);
        db.insert(hash, rlp).map_err(|_| Error::Database)?;
        Ok(hash)
    }
}

impl Default for Trie {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// RLP encoding
// ============================================================

fn rlp_encode_node(node: &Node) -> Vec<u8> {
    match node {
        Node::Empty => encode_str(b""),
        Node::Leaf { path, value } => {
            let path_bytes = hp_encode(path.as_nibbles(), true);
            let encoded_path = encode_str(&path_bytes);
            let encoded_value = encode_str(value);
            encode_list(&[&encoded_path, &encoded_value])
        }
        Node::Extension { path, child } => {
            let path_bytes = hp_encode(path.as_nibbles(), false);
            let encoded_path = encode_str(&path_bytes);
            let encoded_child = rlp_encode_ref(child);
            encode_list(&[&encoded_path, &encoded_child])
        }
        Node::Branch { children, value } => {
            let mut items = Vec::with_capacity(17);
            for child in children.iter() {
                match child {
                    Some(nr) => items.push(rlp_encode_ref(nr)),
                    None => items.push(encode_str(b"")),
                }
            }
            items.push(match value {
                Some(v) => encode_str(v),
                None => encode_str(b""),
            });
            let refs: Vec<&[u8]> = items.iter().map(Vec::as_slice).collect();
            encode_list(&refs)
        }
    }
}

fn rlp_encode_ref(node_ref: &NodeRef) -> Vec<u8> {
    match node_ref {
        NodeRef::Empty => encode_str(b""),
        NodeRef::Hash(h) => encode_str(&h[..]),
        NodeRef::Inline(n) => rlp_encode_node(n),
    }
}

// ============================================================
// RLP decoding
// ============================================================

fn decode_node(data: &[u8]) -> Result<Node, Error> {
    let item = decode_strict(data).map_err(|_| Error::DecodeFailed)?;
    decode_node_from_item(&item)
}

fn decode_node_from_item(item: &RlpItem) -> Result<Node, Error> {
    match item {
        RlpItem::Str(s) => {
            if s.is_empty() {
                Ok(Node::Empty)
            } else {
                Err(Error::DecodeFailed)
            }
        }
        RlpItem::List(items) => {
            if items.len() == 2 {
                let path_data = match &items[0] {
                    RlpItem::Str(s) => *s,
                    _ => return Err(Error::DecodeFailed),
                };
                let (nibbles, is_leaf) = hp_decode(path_data);
                if is_leaf {
                    let value = match &items[1] {
                        RlpItem::Str(s) => s.to_vec(),
                        _ => return Err(Error::DecodeFailed),
                    };
                    Ok(Node::Leaf {
                        path: NibbleBuf::from_nibbles(&nibbles),
                        value,
                    })
                } else {
                    let child = decode_ref_from_item(&items[1])?;
                    Ok(Node::Extension {
                        path: NibbleBuf::from_nibbles(&nibbles),
                        child: Box::new(child),
                    })
                }
            } else if items.len() == 17 {
                let mut children = empty_children();
                for i in 0..16 {
                    match decode_ref_from_item(&items[i])? {
                        NodeRef::Empty => {}
                        nr => children[i] = Some(Box::new(nr)),
                    }
                }
                let value = match &items[16] {
                    RlpItem::Str(s) => {
                        if s.is_empty() {
                            None
                        } else {
                            Some(s.to_vec())
                        }
                    }
                    _ => return Err(Error::DecodeFailed),
                };
                Ok(Node::Branch { children, value })
            } else {
                Err(Error::DecodeFailed)
            }
        }
    }
}

fn decode_ref_from_item(item: &RlpItem) -> Result<NodeRef, Error> {
    match item {
        RlpItem::Str(s) => {
            if s.is_empty() {
                Ok(NodeRef::Empty)
            } else if s.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(s);
                Ok(NodeRef::Hash(arr))
            } else {
                let node = decode_node_from_item(item)?;
                Ok(NodeRef::Inline(Box::new(node)))
            }
        }
        RlpItem::List(_) => {
            let node = decode_node_from_item(item)?;
            Ok(NodeRef::Inline(Box::new(node)))
        }
    }
}

// ============================================================
// Node commitment (encode → hash → store)
// ============================================================

/// RLP-encode a node, store it in the DB, return its reference.
/// Nodes < 32 bytes are returned as Inline (embedded in the parent node's RLP).
fn commit_node(db: &mut dyn super::db::Database, node: Node) -> Result<NodeRef, Error> {
    match node {
        Node::Empty => Ok(NodeRef::Empty),
        non_empty => {
            let rlp = rlp_encode_node(&non_empty);
            if rlp.len() < 32 {
                return Ok(NodeRef::Inline(Box::new(non_empty)));
            }
            let hash = keccak256(&rlp);
            db.insert(hash, rlp).map_err(|_| Error::Database)?;
            Ok(NodeRef::Hash(hash))
        }
    }
}

// ============================================================
// Node resolution (DB → decoded)
// ============================================================

fn resolve_ref(db: &dyn super::db::Database, node_ref: &NodeRef) -> Result<Node, Error> {
    match node_ref {
        NodeRef::Empty => Ok(Node::Empty),
        NodeRef::Inline(n) => Ok(*n.clone()),
        NodeRef::Hash(hash) => {
            let data = db
                .get(hash)
                .map_err(|_| Error::Database)?
                .ok_or(Error::MissingNode)?;
            decode_node(&data)
        }
    }
}

// ============================================================
// Helpers
// ============================================================

fn common_prefix_len(a: &[u8], b: &[u8]) -> usize {
    let max = min(a.len(), b.len());
    for i in 0..max {
        if a[i] != b[i] {
            return i;
        }
    }
    max
}

// ============================================================
// Get internal
// ============================================================

fn get_internal(
    db: &dyn super::db::Database,
    node: &Node,
    path: &[u8],
) -> Result<Option<Vec<u8>>, Error> {
    match node {
        Node::Empty => Ok(None),
        Node::Leaf { path: lp, value } => {
            if lp.as_nibbles() == path {
                Ok(Some(value.clone()))
            } else {
                Ok(None)
            }
        }
        Node::Extension { path: ep, child } => {
            let ep_slice = ep.as_nibbles();
            if path.starts_with(ep_slice) {
                let child_node = resolve_ref(db, child)?;
                get_internal(db, &child_node, &path[ep_slice.len()..])
            } else {
                Ok(None)
            }
        }
        Node::Branch { children, value } => {
            if path.is_empty() {
                return Ok(value.clone());
            }
            let nibble = path[0] as usize;
            match &children[nibble] {
                Some(child) => {
                    let child_node = resolve_ref(db, child)?;
                    get_internal(db, &child_node, &path[1..])
                }
                None => Ok(None),
            }
        }
    }
}

// ============================================================
// Insert internal
// ============================================================

fn insert_internal(
    db: &mut dyn super::db::Database,
    node: Node,
    path: &[u8],
    value: Vec<u8>,
) -> Result<Node, Error> {
    match node {
        // Empty → create leaf
        Node::Empty => Ok(Node::Leaf {
            path: NibbleBuf::from_nibbles(path),
            value,
        }),

        // Leaf → either update, no-op, or split
        Node::Leaf {
            path: ref lp,
            value: ref lv,
        } => {
            let lp_slice = lp.as_nibbles();
            let common = common_prefix_len(lp_slice, path);

            if common == lp_slice.len() && common == path.len() {
                // Full match
                if *lv == value {
                    return Ok(Node::Leaf {
                        path: *lp,
                        value: lv.clone(),
                    });
                }
                return Ok(Node::Leaf { path: *lp, value });
            }

            let mut children = empty_children();
            let mut branch_value = None::<Vec<u8>>;

            // Existing leaf suffix
            if common < lp_slice.len() {
                let suffix = &lp_slice[common..];
                let child = if suffix.len() == 1 {
                    Node::Leaf {
                        path: NibbleBuf::default(),
                        value: lv.clone(),
                    }
                } else {
                    Node::Leaf {
                        path: NibbleBuf::from_nibbles(&suffix[1..]),
                        value: lv.clone(),
                    }
                };
                children[suffix[0] as usize] = Some(Box::new(commit_node(db, child)?));
            } else {
                branch_value = Some(lv.clone());
            }

            // New value suffix
            if common < path.len() {
                let suffix = &path[common..];
                let child = if suffix.len() == 1 {
                    Node::Leaf {
                        path: NibbleBuf::default(),
                        value,
                    }
                } else {
                    let sub_path = &suffix[1..];
                    insert_internal(db, Node::Empty, sub_path, value)?
                };
                children[suffix[0] as usize] = Some(Box::new(commit_node(db, child)?));
            } else {
                branch_value = Some(value);
            }

            let branch = Node::Branch {
                children,
                value: branch_value,
            };
            if common == 0 {
                return Ok(branch);
            }
            Ok(Node::Extension {
                path: NibbleBuf::from_nibbles(&path[..common]),
                child: Box::new(commit_node(db, branch)?),
            })
        }

        // Extension → either recurse or split
        Node::Extension {
            path: ref ep,
            child: ref ec,
        } => {
            let ep_slice = ep.as_nibbles();
            let common = common_prefix_len(ep_slice, path);

            if common == ep_slice.len() {
                // Full extension match → recurse into child
                let resolved = resolve_ref(db, ec)?;
                let new_child = insert_internal(db, resolved, &path[common..], value)?;

                // Merge if child is also an Extension or Leaf
                return Ok(match new_child {
                    Node::Extension {
                        path: inner_path,
                        child: inner_child,
                    } => Node::Extension {
                        path: ep.merge(&inner_path),
                        child: inner_child,
                    },
                    Node::Leaf {
                        path: inner_path,
                        value: inner_value,
                    } => Node::Leaf {
                        path: ep.merge(&inner_path),
                        value: inner_value,
                    },
                    other => Node::Extension {
                        path: *ep,
                        child: Box::new(commit_node(db, other)?),
                    },
                });
            }

            // Partial match → split extension
            let mut children = empty_children();
            let mut branch_value = None::<Vec<u8>>;

            // Extension's remaining path
            if common < ep_slice.len() {
                let suffix = &ep_slice[common..];
                if suffix.len() == 1 {
                    children[suffix[0] as usize] = Some(ec.clone());
                } else {
                    let child = Node::Extension {
                        path: NibbleBuf::from_nibbles(&suffix[1..]),
                        child: ec.clone(),
                    };
                    children[suffix[0] as usize] = Some(Box::new(commit_node(db, child)?));
                }
            }

            // New key's remaining path
            if common < path.len() {
                let suffix = &path[common..];
                let child = if suffix.len() == 1 {
                    insert_internal(db, Node::Empty, &[], value)?
                } else {
                    insert_internal(db, Node::Empty, &suffix[1..], value)?
                };
                children[suffix[0] as usize] = Some(Box::new(commit_node(db, child)?));
            } else {
                branch_value = Some(value);
            }

            let branch = Node::Branch {
                children,
                value: branch_value,
            };
            if common == 0 {
                return Ok(branch);
            }
            Ok(Node::Extension {
                path: NibbleBuf::from_nibbles(&path[..common]),
                child: Box::new(commit_node(db, branch)?),
            })
        }

        // Branch → update value or recurse into child
        Node::Branch {
            children: mut br_children,
            value: br_value,
        } => {
            if path.is_empty() {
                if br_value.as_ref() == Some(&value) {
                    return Ok(Node::Branch {
                        children: br_children,
                        value: br_value,
                    });
                }
                return Ok(Node::Branch {
                    children: br_children,
                    value: Some(value),
                });
            }

            let nibble = path[0] as usize;
            let rest = &path[1..];

            let existing = match &br_children[nibble] {
                Some(child) => resolve_ref(db, child)?,
                None => Node::Empty,
            };
            let new_child = insert_internal(db, existing, rest, value)?;
            br_children[nibble] = Some(Box::new(commit_node(db, new_child)?));

            Ok(Node::Branch {
                children: br_children,
                value: br_value,
            })
        }
    }
}

// ============================================================
// Remove internal
// ============================================================

fn remove_internal(
    db: &mut dyn super::db::Database,
    node: Node,
    path: &[u8],
) -> Result<Node, Error> {
    match node {
        Node::Empty => Ok(Node::Empty),

        Node::Leaf { path: ref lp, .. } => {
            if lp.as_nibbles() == path {
                Ok(Node::Empty)
            } else {
                Ok(node)
            }
        }

        Node::Extension {
            path: ref ep,
            child: ref ec,
        } => {
            let ep_slice = ep.as_nibbles();
            if path.starts_with(ep_slice) {
                let resolved = resolve_ref(db, ec)?;
                let new_child = remove_internal(db, resolved, &path[ep_slice.len()..])?;

                if new_child == Node::Empty {
                    return Ok(Node::Empty);
                }

                // Try to merge
                return Ok(match new_child {
                    Node::Extension {
                        path: inner_path,
                        child: inner_child,
                    } => Node::Extension {
                        path: ep.merge(&inner_path),
                        child: inner_child,
                    },
                    Node::Leaf {
                        path: inner_path,
                        value: inner_value,
                    } => Node::Leaf {
                        path: ep.merge(&inner_path),
                        value: inner_value,
                    },
                    other => Node::Extension {
                        path: *ep,
                        child: Box::new(commit_node(db, other)?),
                    },
                });
            }
            // key not in this subtrie
            Ok(node)
        }

        Node::Branch {
            children: mut br_children,
            value: br_value,
        } => {
            if path.is_empty() {
                if br_value.is_none() {
                    return Ok(Node::Branch {
                        children: br_children,
                        value: None,
                    });
                }
                // Remove value, possibly collapse
                return cleanse_branch(db, br_children, None);
            }

            let nibble = path[0] as usize;
            let rest = &path[1..];

            let existing = match &br_children[nibble] {
                Some(child) => resolve_ref(db, child)?,
                None => {
                    return Ok(Node::Branch {
                        children: br_children,
                        value: br_value,
                    })
                }
            };
            let new_child = remove_internal(db, existing, rest)?;

            if new_child == Node::Empty {
                br_children[nibble] = None;
            } else {
                br_children[nibble] = Some(Box::new(commit_node(db, new_child)?));
            }

            cleanse_branch(db, br_children, br_value)
        }
    }
}

// ============================================================
// Branch cleansing — collapse single-child branches
// ============================================================

fn cleanse_branch(
    db: &mut dyn super::db::Database,
    children: [Option<Box<NodeRef>>; 16],
    value: Option<Vec<u8>>,
) -> Result<Node, Error> {
    let mut count = 0usize;
    let mut last_idx = 0usize;
    for (i, child) in children.iter().enumerate() {
        if child.is_some() {
            count += 1;
            last_idx = i;
        }
    }

    match (count, value) {
        (0, None) => Ok(Node::Empty),

        (0, Some(v)) => Ok(Node::Leaf {
            path: NibbleBuf::default(),
            value: v,
        }),

        (1, None) => {
            let child_ref = children[last_idx]
                .as_ref()
                .expect("count == 1 guarantees a child exists");
            let resolved = resolve_ref(db, child_ref)?;
            let idx_byte = last_idx as u8;

            match resolved {
                Node::Extension {
                    path: cp,
                    child: cc,
                } => {
                    let merged_path = NibbleBuf::from_nibbles(&[idx_byte]).merge(&cp);
                    Ok(Node::Extension {
                        path: merged_path,
                        child: cc,
                    })
                }
                Node::Leaf {
                    path: cp,
                    value: cv,
                } => {
                    let merged_path = NibbleBuf::from_nibbles(&[idx_byte]).merge(&cp);
                    Ok(Node::Leaf {
                        path: merged_path,
                        value: cv,
                    })
                }
                other => {
                    let committed = commit_node(db, other)?;
                    Ok(Node::Extension {
                        path: NibbleBuf::from_nibbles(&[idx_byte]),
                        child: Box::new(committed),
                    })
                }
            }
        }

        (_, opt_val) => Ok(Node::Branch {
            children,
            value: opt_val,
        }),
    }
}

// ============================================================
// NibbleBuf merge helper
// ============================================================

impl NibbleBuf {
    /// Concatenate two nibble paths.
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        debug_assert!(
            self.len + other.len <= MAX_NIBBLES,
            "NibbleBuf::merge overflow: {} + {} > {}",
            self.len,
            other.len,
            MAX_NIBBLES
        );
        let mut inner = [0u8; MAX_NIBBLES];
        let self_end = min(self.len, MAX_NIBBLES);
        inner[..self_end].copy_from_slice(&self.inner[..self_end]);
        let remaining = MAX_NIBBLES - self_end;
        let other_len = min(other.len, remaining);
        let other_start = self_end;
        inner[other_start..other_start + other_len].copy_from_slice(&other.inner[..other_len]);
        Self {
            inner,
            len: self_end + other_len,
        }
    }
}

// ============================================================
// Storage trie pruning
// ============================================================

/// Recursively delete all trie nodes reachable from a root hash.
/// Used to clean up stale storage trie nodes when an account is deleted.
pub fn delete_trie_nodes(db: &mut dyn super::db::Database, root: &[u8; 32]) -> Result<(), Error> {
    if *root == EMPTY_ROOT_HASH {
        return Ok(());
    }
    let data = db
        .get(root)
        .map_err(|_| Error::Database)?
        .ok_or(Error::MissingNode)?;
    let node = decode_node(&data)?;
    match node {
        Node::Branch { children, value: _ } => {
            for child in children.iter().flatten() {
                if let NodeRef::Hash(h) = child.as_ref() {
                    delete_trie_nodes(db, h)?;
                }
            }
        }
        Node::Extension { path: _, child } => {
            if let NodeRef::Hash(h) = child.as_ref() {
                delete_trie_nodes(db, h)?;
            }
        }
        Node::Leaf { .. } | Node::Empty => {}
    }
    db.remove(root);
    Ok(())
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::MemoryDB;

    #[test]
    fn empty_trie_root() {
        let db = &mut MemoryDB::new();
        let trie = Trie::new();
        assert_eq!(trie.root_hash(db).unwrap(), EMPTY_ROOT_HASH);
    }

    #[test]
    fn from_root_empty() {
        let db = &mut MemoryDB::new();
        let trie = Trie::from_root(db, &EMPTY_ROOT_HASH).unwrap();
        assert_eq!(trie.root_hash(db).unwrap(), EMPTY_ROOT_HASH);
    }

    #[test]
    fn insert_and_get() {
        let db = &mut MemoryDB::new();
        let mut trie = Trie::new();
        trie.insert(db, b"dog", b"puppy".to_vec()).unwrap();
        assert_eq!(trie.get(db, b"dog").unwrap(), Some(b"puppy".to_vec()));
        assert_eq!(trie.get(db, b"cat").unwrap(), None);
    }

    #[test]
    fn insert_noop() {
        let db = &mut MemoryDB::new();
        let mut trie = Trie::new();
        trie.insert(db, b"dog", b"puppy".to_vec()).unwrap();
        let h1 = trie.root_hash(db).unwrap();
        trie.insert(db, b"dog", b"puppy".to_vec()).unwrap();
        let h2 = trie.root_hash(db).unwrap();
        assert_eq!(h1, h2, "no-op insert must not change root hash");
    }

    #[test]
    fn insert_update_value() {
        let db = &mut MemoryDB::new();
        let mut trie = Trie::new();
        trie.insert(db, b"dog", b"puppy".to_vec()).unwrap();
        trie.insert(db, b"dog", b"doggy".to_vec()).unwrap();
        assert_eq!(trie.get(db, b"dog").unwrap(), Some(b"doggy".to_vec()));
    }

    #[test]
    fn insert_two_keys() {
        let db = &mut MemoryDB::new();
        let mut trie = Trie::new();
        trie.insert(db, b"dog", b"puppy".to_vec()).unwrap();
        trie.insert(db, b"cat", b"kitten".to_vec()).unwrap();
        assert_eq!(trie.get(db, b"dog").unwrap(), Some(b"puppy".to_vec()));
        assert_eq!(trie.get(db, b"cat").unwrap(), Some(b"kitten".to_vec()));
    }

    #[test]
    fn insert_remove() {
        let db = &mut MemoryDB::new();
        let mut trie = Trie::new();
        trie.insert(db, b"dog", b"puppy".to_vec()).unwrap();
        trie.remove(db, b"dog").unwrap();
        assert_eq!(trie.get(db, b"dog").unwrap(), None);
        assert_eq!(trie.root_hash(db).unwrap(), EMPTY_ROOT_HASH);
    }

    #[test]
    fn insert_two_remove_one() {
        let db = &mut MemoryDB::new();
        let mut trie = Trie::new();
        trie.insert(db, b"dog", b"puppy".to_vec()).unwrap();
        trie.insert(db, b"cat", b"kitten".to_vec()).unwrap();
        trie.remove(db, b"dog").unwrap();
        assert_eq!(trie.get(db, b"dog").unwrap(), None);
        assert_eq!(trie.get(db, b"cat").unwrap(), Some(b"kitten".to_vec()));
    }

    #[test]
    fn insert_many_deterministic_root() {
        let db = &mut MemoryDB::new();
        let mut trie = Trie::new();
        let keys: [(&[u8], &[u8]); 5] = [
            (b"do", b"verb"),
            (b"dog", b"puppy"),
            (b"doge", b"meme"),
            (b"horse", b"stallion"),
            (b"doggo", b"pupper"),
        ];
        for (k, v) in &keys {
            trie.insert(db, k, v.to_vec()).unwrap();
        }
        let root = trie.root_hash(db).unwrap();
        let mut trie2 = Trie::new();
        for (k, v) in &keys {
            trie2.insert(db, k, v.to_vec()).unwrap();
        }
        assert_eq!(trie2.root_hash(db).unwrap(), root);
    }

    #[test]
    fn get_nonexistent_key() {
        let db = &mut MemoryDB::new();
        let mut trie = Trie::new();
        trie.insert(db, b"dog", b"puppy".to_vec()).unwrap();
        assert_eq!(trie.get(db, b"horse").unwrap(), None);
    }

    #[test]
    fn remove_nonexistent_key() {
        let db = &mut MemoryDB::new();
        let mut trie = Trie::new();
        trie.insert(db, b"dog", b"puppy".to_vec()).unwrap();
        trie.remove(db, b"horse").unwrap();
        assert_eq!(trie.get(db, b"dog").unwrap(), Some(b"puppy".to_vec()));
    }

    #[test]
    fn from_root_persistence() {
        let db = &mut MemoryDB::new();
        let mut trie = Trie::new();
        trie.insert(db, b"dog", b"puppy".to_vec()).unwrap();
        let h = trie.root_hash(db).unwrap();

        let trie2 = Trie::from_root(db, &h).unwrap();
        assert_eq!(trie2.get(db, b"dog").unwrap(), Some(b"puppy".to_vec()));
    }

    #[test]
    fn long_key_insert() {
        let db = &mut MemoryDB::new();
        let mut trie = Trie::new();
        let key = b"this is a very long key that will produce many nibbles for the trie to process";
        trie.insert(db, key, b"value".to_vec()).unwrap();
        assert_eq!(trie.get(db, key).unwrap(), Some(b"value".to_vec()));
    }

    #[test]
    fn empty_value() {
        let db = &mut MemoryDB::new();
        let mut trie = Trie::new();
        trie.insert(db, b"key", Vec::new()).unwrap();
        assert_eq!(trie.get(db, b"key").unwrap(), Some(Vec::new()));
    }

    #[test]
    fn remove_branch_collapse() {
        let db = &mut MemoryDB::new();
        let mut trie = Trie::new();
        trie.insert(db, b"a", b"val_a".to_vec()).unwrap();
        trie.insert(db, b"b", b"val_b".to_vec()).unwrap();
        trie.remove(db, b"a").unwrap();
        assert_eq!(trie.get(db, b"a").unwrap(), None);
        assert_eq!(trie.get(db, b"b").unwrap(), Some(b"val_b".to_vec()));
    }

    #[test]
    fn insert_update_branch_value() {
        let db = &mut MemoryDB::new();
        let mut trie = Trie::new();
        trie.insert(db, b"do", b"verb".to_vec()).unwrap();
        trie.insert(db, b"dog", b"puppy".to_vec()).unwrap();
        trie.insert(db, b"do", b"action".to_vec()).unwrap();
        assert_eq!(trie.get(db, b"do").unwrap(), Some(b"action".to_vec()));
        assert_eq!(trie.get(db, b"dog").unwrap(), Some(b"puppy".to_vec()));
    }

    #[test]
    fn remove_deep_extension() {
        let db = &mut MemoryDB::new();
        let mut trie = Trie::new();
        trie.insert(db, b"abcd", b"val_d".to_vec()).unwrap();
        trie.insert(db, b"abce", b"val_e".to_vec()).unwrap();
        trie.remove(db, b"abcd").unwrap();
        assert_eq!(trie.get(db, b"abcd").unwrap(), None);
        assert_eq!(trie.get(db, b"abce").unwrap(), Some(b"val_e".to_vec()));
    }

    /// Regression: inserting a prefix key after a longer key must set the
    /// branch value (extension handler's `common == path.len()` branch).
    #[test]
    fn insert_prefix_key_after_longer_key() {
        let db = &mut MemoryDB::new();
        let mut trie = Trie::new();
        trie.insert(db, b"dog", b"puppy".to_vec()).unwrap();
        trie.insert(db, b"do", b"verb".to_vec()).unwrap();
        assert_eq!(trie.get(db, b"do").unwrap(), Some(b"verb".to_vec()));
        assert_eq!(trie.get(db, b"dog").unwrap(), Some(b"puppy".to_vec()));
    }

    #[test]
    fn inline_node_skips_db() {
        let db = &mut MemoryDB::new();
        let mut trie = Trie::new();
        // Single leaf with short key+value → RLP < 32 bytes → Inline
        trie.insert(db, b"a", b"1".to_vec()).unwrap();
        let root = trie.root_hash(db).unwrap();
        let trie2 = Trie::from_root(db, &root).unwrap();
        assert_eq!(trie2.get(db, b"a").unwrap(), Some(b"1".to_vec()));
    }
}

#![no_std]
#![deny(unsafe_code)]
extern crate alloc;

pub mod db;
pub mod trie;

pub use db::{Database, MemoryDB};
pub use trie::{delete_trie_nodes, Error, Node, NodeRef, Trie, EMPTY_ROOT_HASH};

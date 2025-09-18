//! Merkle Patricia Tries.

use std::fmt::Debug;
use std::hash::Hash;
use std::io::{Read, Write};
use std::ops::Deref;

use crate::DefaultDB;
use crate::Storable;
use crate::arena::{ArenaKey, Sp};
use crate::db::DB;
use crate::serialize::{self, Deserializable, Serializable, Version, Versioned};
use crate::storable::Loader;
use crate::storage::default_storage;
use derive_where::derive_where;

/// A Merkle Patricia Trie
#[derive_where(Debug, Eq, Clone, PartialEq; V)]
#[derive(Storable)]
#[storable(db = D)]
pub struct MerklePatriciaTrie<V: Storable<D>, D: DB = DefaultDB>(pub(crate) Sp<Node<V, D>, D>);

impl<V: Storable<D>, D: DB> Default for MerklePatriciaTrie<V, D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Storable<D>, D: DB> MerklePatriciaTrie<V, D> {
    /// Construct an empty trie
    pub fn new() -> Self {
        MerklePatriciaTrie(Sp::new(Node::Empty))
    }

    /// Insert a value into the trie
    pub fn insert(&self, path: &[u8], value: V) -> Self {
        MerklePatriciaTrie(self.0.insert(path, value))
    }

    /// Lookup a value in the trie
    pub fn lookup(&self, path: &[u8]) -> Option<&V> {
        self.0.lookup(path)
    }

    /// Lookup a value in the trie
    pub fn lookup_sp(&self, path: &[u8]) -> Option<Sp<V, D>> {
        self.0.lookup_sp(path)
    }

    /// Remove a value from the trie
    pub fn remove(&self, path: &[u8]) -> Self {
        MerklePatriciaTrie(self.0.remove(path))
    }

    /// Generate iterator over leaves, (path, &value)
    pub fn iter(&self) -> MPTIter<V, D> {
        MPTIter(self.0.leaves(&[]).into_iter())
    }

    /// Get the number of leaves in a trie
    pub fn size(&mut self) -> usize {
        self.0.size()
    }

    /// Return true if the trie is empty, false otherwise
    pub fn is_empty(&self) -> bool {
        matches!(self.0.deref(), Node::Empty)
    }
}

impl<V: Storable<D>, D: DB> Hash for MerklePatriciaTrie<V, D> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        <Sp<Node<V, D>, D> as Hash>::hash(&self.0, state)
    }

    fn hash_slice<H: std::hash::Hasher>(data: &[Self], state: &mut H)
    where
        Self: Sized,
    {
        Sp::<Node<V, D>, D>::hash_slice(
            &data
                .iter()
                .map(|d| d.0.clone())
                .collect::<Vec<Sp<Node<V, D>, D>>>(),
            state,
        )
    }
}

impl<V: Storable<D> + Ord, D: DB> Ord for MerklePatriciaTrie<V, D> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<V: Storable<D> + PartialOrd, D: DB> PartialOrd for MerklePatriciaTrie<V, D> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<V: Storable<D>, D: DB> Versioned for MerklePatriciaTrie<V, D> {
    const VERSION: Option<Version> = Some(Version { major: 1, minor: 1 });
}

impl<V: Storable<D>, D: DB> Serializable for MerklePatriciaTrie<V, D> {
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        Sp::<Node<V, D>, D>::serialize(&value.0, writer)
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        Sp::<Node<V, D>, D>::serialized_size(&value.0)
    }
}

impl<V: Storable<D>, D: DB> Deserializable for MerklePatriciaTrie<V, D> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursive_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 1, minor: 1 }) => Ok(MerklePatriciaTrie(
                default_storage()
                    .arena
                    .clone()
                    .deserialize_sp(reader, recursive_depth)?,
            )),
            Some(Version { major: 1, minor: 0 }) => Ok(MerklePatriciaTrie(
                default_storage()
                    .arena
                    .clone()
                    .deserialize_sp_1_0(reader, recursive_depth)?,
            )),
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version".to_string(),
            )),
        }
    }
}

/// Iterator over (path, value) pairs in MerklePatriciaTrie
pub struct MPTIter<T: Storable<D> + 'static, D: DB>(std::vec::IntoIter<(Vec<u8>, Sp<T, D>)>);

impl<T: Storable<D>, D: DB> Iterator for MPTIter<T, D> {
    type Item = (Vec<u8>, Sp<T, D>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

// The MPT node
#[derive(Debug, Default)]
#[derive_where(Clone, Hash, PartialEq, Eq; T)]
#[derive_where(PartialOrd; T: PartialOrd)]
#[derive_where(Ord; T: Ord)]
pub(crate) enum Node<T: Storable<D> + 'static, D: DB> {
    #[default]
    Empty,
    Leaf {
        value: Sp<T, D>,
    },
    Branch {
        children: [Sp<Node<T, D>, D>; 16],
    },
    Extension {
        compressed_path: Vec<u8>, // with a length no longer than 255
        child: Sp<Node<T, D>, D>,
    },
    MidBranchLeaf {
        value: Sp<T, D>,
        child: Sp<Node<T, D>, D>, // should only be an `Extension` or `Branch`
    },
}

impl<T: Storable<D>, D: DB> Versioned for Node<T, D> {
    const VERSION: Option<serialize::Version> = Some(Version { major: 1, minor: 1 });
}

fn compress_nibbles(nibbles: &[u8]) -> Vec<u8> {
    let mut compressed: Vec<u8> = vec![0; (nibbles.len() / 2) + (nibbles.len() % 2)];
    for i in 0..nibbles.len() {
        compressed[i / 2] |= nibbles[i] << ((i % 2) * 4);
    }

    compressed
}

fn expand_nibbles(compressed: &[u8], len: usize) -> Vec<u8> {
    let mut nibbles = Vec::new();
    for i in 0..len {
        if i % 2 == 0 {
            nibbles.push(compressed[i / 2] & 0x0f);
        } else {
            nibbles.push((compressed[i / 2] & 0xf0) >> 4);
        }
    }

    nibbles
}

impl<T: Storable<D>, D: DB> Sp<Node<T, D>, D> {
    fn lookup_sp(&self, path: &[u8]) -> Option<Sp<T, D>> {
        self.lookup_with(path, Clone::clone)
    }

    fn lookup<'a>(&'a self, path: &[u8]) -> Option<&'a T> {
        self.lookup_with::<&T>(path, |sp| sp.deref())
    }

    fn lookup_with<'a, S>(&'a self, path: &[u8], f: impl FnOnce(&'a Sp<T, D>) -> S) -> Option<S> {
        match self.deref() {
            Node::Empty => None,
            Node::Leaf { value, .. } if path.is_empty() => Some(f(value)),
            // If the path isn't empty
            Node::Leaf { .. } => None,
            Node::Branch { children, .. } => {
                let index: usize = path[0].into();
                children[index].lookup_with(&path[1..], f)
            }
            Node::Extension {
                compressed_path,
                child,
                ..
            } => {
                for i in 0..compressed_path.len() {
                    if compressed_path[i] != path[i] {
                        return None;
                    }
                }
                child.lookup_with(&path[compressed_path.len()..], f)
            }
            Node::MidBranchLeaf { value, child } => {
                if path.is_empty() {
                    Some(f(value))
                } else {
                    child.lookup_with(path, f)
                }
            }
        }
    }

    fn insert(&self, path: &[u8], value: T) -> Self {
        if path.is_empty() {
            let value = self.arena.alloc(value);
            let node = match self.deref() {
                Node::Empty | Node::Leaf { .. } => Node::Leaf { value },
                Node::Branch { .. } | Node::Extension { .. } => Node::MidBranchLeaf {
                    value,
                    child: self.clone(),
                },
                Node::MidBranchLeaf { child, .. } => Node::MidBranchLeaf {
                    value,
                    child: child.clone(),
                },
            };
            return self.arena.alloc(node);
        }

        match self.deref() {
            Node::Empty => {
                let mut child = self.arena.alloc(Node::Leaf {
                    value: self.arena.alloc(value),
                });

                for working_path in path.chunks(255).rev() {
                    child = self.arena.alloc(Node::Extension {
                        compressed_path: working_path.to_vec(),
                        child: child.clone(),
                    });
                }
                child
            }
            Node::Leaf { value: self_value } => {
                let mut child = self.arena.alloc(Node::Leaf {
                    value: self.arena.alloc(value),
                });
                for chunk in path.chunks(255).rev() {
                    child = self.arena.alloc(Node::Extension {
                        compressed_path: chunk.to_vec(),
                        child,
                    });
                }

                self.arena.alloc(Node::MidBranchLeaf {
                    value: self_value.clone(),
                    child,
                })
            }
            Node::Branch { children, .. } => {
                let index: usize = path[0].into();
                let mut new_children = children.clone();
                new_children[index] = new_children[index].insert(&path[1..], value);
                self.arena.alloc(Node::Branch {
                    children: new_children,
                })
            }
            Node::Extension {
                compressed_path,
                child,
                ..
            } => {
                let working_path: Vec<u8> =
                    path.chunks(255).next().expect("path is not empty").to_vec();

                let index = compressed_path
                    .iter()
                    .zip(working_path)
                    .take_while(|(a, b)| **a == *b)
                    .count();

                // complete path match, insert at the child node
                if index == compressed_path.len() {
                    let new_child = child.insert(&path[index..], value);
                    self.arena.alloc(Node::Extension {
                        compressed_path: compressed_path.clone(),
                        child: new_child,
                    })
                } else {
                    // if path splits on final nibble old child node doesn't need an extension,
                    // otherwise it does
                    let remaining = if index == compressed_path.len() - 1 {
                        child.clone()
                    } else {
                        self.arena.alloc(Node::Extension {
                            compressed_path: compressed_path[(index + 1)..].to_vec(),
                            child: child.clone(),
                        })
                    };

                    let compressed_path_index: usize = compressed_path[index].into();
                    let mut children: [Sp<Node<T, D>, D>; 16] =
                        core::array::from_fn(|_| self.arena.alloc(Node::Empty));
                    children[compressed_path_index] = remaining;

                    let mut branch = self.arena.alloc(Node::Branch { children });

                    branch = branch.insert(&path[index..], value);

                    // if path split on first nibble no extension required, otherwise it is
                    if index == 0 {
                        branch
                    } else {
                        self.arena.alloc(Node::Extension {
                            compressed_path: compressed_path[0..index].to_vec(),
                            child: branch,
                        })
                    }
                }
            }
            Node::MidBranchLeaf {
                child,
                value: leaf_value,
            } => self.arena.alloc(Node::MidBranchLeaf {
                value: leaf_value.clone(),
                child: child.insert(path, value),
            }),
        }
    }

    fn size(&self) -> usize {
        match self.deref() {
            Node::Empty => 0,
            Node::Leaf { .. } => 1,
            Node::Extension { child, .. } => child.size(),
            Node::Branch { children, .. } => children.iter().map(|c| c.size()).sum(),
            Node::MidBranchLeaf { child, .. } => child.size() + 1,
        }
    }

    fn leaves(&self, current_path: &[u8]) -> Vec<(Vec<u8>, Sp<T, D>)> {
        match self.deref() {
            Node::Empty => Vec::new(),
            Node::Leaf { value, .. } => vec![(current_path.to_vec(), value.clone())],
            Node::Extension {
                compressed_path,
                child,
                ..
            } => {
                let mut new_path = current_path.to_vec();
                new_path.append(&mut compressed_path.clone());
                child.leaves(new_path.as_slice())
            }
            Node::Branch { children, .. } => {
                let mut leaves = Vec::new();
                for (i, child) in children.iter().enumerate() {
                    let mut new_path = current_path.to_vec();
                    new_path.push(i as u8);
                    leaves.extend(child.leaves(new_path.as_slice()));
                }
                leaves
            }
            Node::MidBranchLeaf { value, child } => {
                let mut leaves = child.leaves(current_path);
                leaves.push((current_path.to_vec(), value.clone()));
                leaves
            }
        }
    }

    fn remove(&self, path: &[u8]) -> Self {
        match self.deref() {
            Node::Empty => self.arena.alloc(Node::Empty),
            Node::Leaf { value } => {
                if path.is_empty() {
                    return self.arena.alloc(Node::Empty);
                }

                self.arena.alloc(Node::Leaf {
                    value: value.clone(),
                })
            }
            Node::Branch { children, .. } => {
                let mut new_children = children.clone();
                let index: usize = path[0].into();
                new_children[index] = new_children[index].remove(&path[1..]);

                // Remove branch if only one child remaining
                if new_children
                    .iter()
                    .map(|v| match **v {
                        Node::Empty => 0,
                        _ => 1,
                    })
                    .sum::<usize>()
                    == 1
                {
                    let (only_child_index, only_child) = new_children
                        .iter()
                        .enumerate()
                        .find(|(_i, v)| !matches!(***v, Node::Empty))
                        .unwrap();

                    if let Node::Extension {
                        mut compressed_path,
                        child,
                        ..
                    } = (**only_child).clone()
                    {
                        let mut new_compressed_path = vec![only_child_index as u8];
                        new_compressed_path.append(&mut compressed_path);
                        self.arena.alloc(Node::Extension {
                            compressed_path: new_compressed_path,
                            child,
                        })
                    } else {
                        self.arena.alloc(Node::Extension {
                            compressed_path: vec![only_child_index as u8],
                            child: only_child.clone(),
                        })
                    }
                } else {
                    self.arena.alloc(Node::Branch {
                        children: new_children,
                    })
                }
            }
            Node::Extension {
                compressed_path,
                child,
                ..
            } => {
                for i in 0..compressed_path.len() {
                    if compressed_path[i] != path[i] {
                        return self.arena.alloc(Node::Extension {
                            compressed_path: compressed_path.clone(),
                            child: child.clone(),
                        });
                    }
                }

                let new_child = child.remove(&path[compressed_path.len()..]);
                match new_child.deref() {
                    Node::Empty => self.arena.alloc(Node::Empty),
                    Node::Extension {
                        compressed_path: p,
                        child: c,
                        ..
                    } => {
                        let mut new_compressed_path = compressed_path.clone();
                        new_compressed_path.append(&mut p.clone());

                        let mut child = c.clone();
                        for path_chunk in new_compressed_path.chunks(255).rev() {
                            child = if path_chunk.is_empty() {
                                child
                            } else {
                                self.arena.alloc(Node::Extension {
                                    compressed_path: path_chunk.to_vec(),
                                    child: child.clone(),
                                })
                            };
                        }
                        child
                    }
                    _ => self.arena.alloc(Node::Extension {
                        compressed_path: compressed_path.clone(),
                        child: new_child,
                    }),
                }
            }
            Node::MidBranchLeaf { child, value } => {
                if path.is_empty() {
                    child.clone()
                } else {
                    let child = child.remove(path);
                    match child.deref() {
                        Node::Empty => self.arena.alloc(Node::Leaf {
                            value: value.clone(),
                        }),
                        _ => self.arena.alloc(Node::MidBranchLeaf {
                            value: value.clone(),
                            child: child.remove(path),
                        }),
                    }
                }
            }
        }
    }
}

impl<T: Storable<D> + 'static, D: DB> Storable<D> for Node<T, D> {
    fn children(&self) -> Vec<ArenaKey<D::Hasher>> {
        match self {
            Node::Empty => Vec::new(),
            Node::Leaf { value } => vec![value.root.clone()],
            Node::Branch { children, .. } => children.iter().map(|sp| sp.root.clone()).collect(),
            Node::Extension { child, .. } => vec![child.root.clone()],
            Node::MidBranchLeaf { child, value } => vec![value.root.clone(), child.root.clone()],
        }
    }

    fn to_binary_repr<W: Write>(&self, writer: &mut W) -> Result<(), std::io::Error> {
        match self {
            Node::Empty => {
                u8::serialize(&0, writer)?;
            }
            Node::Leaf { .. } => {
                u8::serialize(&1, writer)?;
            }
            Node::Branch { .. } => {
                u8::serialize(&2, writer)?;
            }
            Node::Extension {
                compressed_path, ..
            } => {
                u8::serialize(&3, writer)?;
                let compressed = compress_nibbles(compressed_path);
                u8::serialize(&(compressed_path.len() as u8), writer)?;
                Vec::<u8>::serialize(&compressed, writer)?;
            }
            Node::MidBranchLeaf { .. } => {
                u8::serialize(&4, writer)?;
            }
        }

        Ok(())
    }

    #[inline(always)]
    fn from_binary_repr<R: Read>(
        reader: &mut R,
        child_hashes: &mut impl Iterator<Item = ArenaKey<D::Hasher>>,
        loader: &impl Loader<D>,
    ) -> Result<Node<T, D>, std::io::Error> {
        let disc = u8::deserialize(reader, 0)?;

        match disc {
            0 => Ok(Node::Empty),
            1 => Ok(Node::Leaf {
                value: loader.get_next(child_hashes)?,
            }),
            2 => {
                let mut children: [Sp<Node<T, D>, D>; 16] =
                    core::array::from_fn(|_| loader.alloc(Node::Empty));
                let mut non_empty_children = 0;
                let mut has_lazy_children = false;

                #[allow(clippy::needless_range_loop)]
                for i in 0..16 {
                    children[i] = loader.get_next(child_hashes)?;
                    if children[i].is_lazy() {
                        has_lazy_children = true;
                    } else if !matches!(*children[i], Node::Empty) {
                        non_empty_children += 1;
                    }
                }

                if !has_lazy_children && non_empty_children < 2 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Fewer than 2 non-empty children in Node::Branch".to_string(),
                    ));
                }

                Ok(Node::Branch { children })
            }
            3 => {
                let len = u8::deserialize(reader, 0)?;
                let path = expand_nibbles(&Vec::<u8>::deserialize(reader, 0)?, len as usize);
                let child: Sp<Node<T, D>, D> = loader.get_next(child_hashes)?;
                if !child.is_lazy()
                    && matches!(child.deref(), Node::Extension { .. })
                    && path.len() != 255
                {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Node::Extension path must be of length 255 when having another Node::Extension child",
                    ));
                }
                Ok(Node::Extension {
                    compressed_path: path,
                    child,
                })
            }
            4 => {
                let value: Sp<T, D> = loader.get_next(child_hashes)?;
                let child: Sp<Node<T, D>, D> = loader.get_next(child_hashes)?;
                if child.is_lazy() {
                    Ok(Node::MidBranchLeaf { value, child })
                } else {
                    match child.deref() {
                        Node::Branch { .. } | Node::Extension { .. } => {
                            Ok(Node::MidBranchLeaf { value, child })
                        }
                        _ => Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Node::MidBranchLeaf may only have Node::Branch or Node::Extension children",
                        )),
                    }
                }
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unrecognised discriminant",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use sha2::Sha256;

    use crate::{
        Storage,
        db::InMemoryDB,
        storage::{WrappedDB, set_default_storage},
    };

    use super::*;
    use crate::serialize::{deserialize, serialize};

    #[test]
    fn insert_lookup() {
        dbg!("start");
        let mut mpt = MerklePatriciaTrie::<u64>::new();
        dbg!("new tree");
        mpt = mpt.insert(&([1, 2, 3]), 100);
        dbg!("inserted 100 at [1, 2, 3]");
        mpt = mpt.insert(&([1, 2, 4]), 104);
        dbg!("inserted 104 at [1, 2, 4]");
        mpt = mpt.insert(&([2, 2, 4]), 105);
        dbg!("inserted 105 at [2, 2, 4]");
        assert_eq!(mpt.lookup(&([1, 2, 3])), Some(&100));
        assert_eq!(mpt.lookup(&([1, 2, 4])), Some(&104));
        assert_eq!(mpt.lookup(&([2, 2, 4])), Some(&105));
    }

    #[test]
    fn remove() {
        let mut mpt = MerklePatriciaTrie::<u64>::new();
        mpt = mpt.insert(&([1, 2, 3]), 100);
        mpt = mpt.insert(&([1, 3, 3]), 102);
        mpt = mpt.remove(&([1, 2, 3]));
        assert_eq!(mpt.size(), 1);
        assert_eq!(mpt.lookup(&([1, 3, 3])), Some(&102));
        assert_eq!(mpt.lookup(&([1, 2, 3])), None);
    }

    #[test]
    fn deduplicate() {
        // Isolate our storage, since we're checking arena size.
        struct Tag;
        type D = WrappedDB<DefaultDB, Tag>;
        let _ = set_default_storage::<D>(Storage::default);
        let mut mpt = MerklePatriciaTrie::<u64, D>::new();
        mpt = mpt.insert(&([1, 2, 3]), 100);
        mpt = mpt.insert(&([1, 2, 2]), 100);
        assert_eq!(mpt.lookup(&([1, 2, 3])), Some(&100));
        assert_eq!(mpt.lookup(&([1, 2, 2])), Some(&100));
        assert_eq!(mpt.size(), 2);
        dbg!(&mpt.0.arena);
        assert_eq!(mpt.0.arena.size(), 5);
    }

    #[test]
    fn mpt_arena_serialization() {
        let mut mpt = MerklePatriciaTrie::<u8>::new();
        mpt = mpt.insert(&([1, 2, 3]), 100);
        mpt = mpt.insert(&([1, 2, 4]), 104);
        mpt = mpt.insert(&([2, 2, 4]), 105);
        let mut bytes = Vec::new();
        MerklePatriciaTrie::serialize(&mpt, &mut bytes).unwrap();
        assert_eq!(bytes.len(), MerklePatriciaTrie::<u8>::serialized_size(&mpt));
        let mpt: MerklePatriciaTrie<u8> =
            MerklePatriciaTrie::deserialize(&mut bytes.as_slice(), 0).unwrap();
        assert_eq!(mpt.lookup(&([1, 2, 3])), Some(&100));
        assert_eq!(mpt.lookup(&([1, 2, 4])), Some(&104));
        assert_eq!(mpt.lookup(&([2, 2, 4])), Some(&105));
    }

    #[test]
    fn nodes_stored() {
        // Isolate our storage, since we're checking arena size.
        struct Tag;
        type D = WrappedDB<DefaultDB, Tag>;
        let _ = set_default_storage::<D>(Storage::default);
        let arena = &default_storage::<D>().arena;
        {
            let mut mpt: MerklePatriciaTrie<u64, D> = MerklePatriciaTrie::new();
            mpt = mpt.insert(&([1, 2, 3]), 100);
            mpt = mpt.insert(&([1, 2, 4]), 104);
            mpt = mpt.insert(&([2, 2, 4]), 105);
            assert_eq!(arena.size(), 11);
            assert_eq!(mpt.lookup(&([2, 2, 4])), Some(&105));
        }
        assert_eq!(arena.size(), 0);
    }

    #[test]
    fn long_extension_paths_serialization() {
        let mut mpt: MerklePatriciaTrie<u8, InMemoryDB<Sha256>> = MerklePatriciaTrie::new();
        mpt = mpt.insert(&(vec![2; 300]), 100);

        let mut bytes = Vec::new();
        serialize(&mpt, &mut bytes, serialize::NetworkId::Undeployed).unwrap();
        let deserialized_mpt: MerklePatriciaTrie<u8> =
            deserialize(&mut bytes.as_slice(), serialize::NetworkId::Undeployed).unwrap();
        assert_eq!(deserialized_mpt, mpt);

        assert_eq!(
            mpt.iter().map(|(k, _)| k).collect::<Vec<Vec<u8>>>(),
            vec![vec![2; 300]]
        );
        assert_eq!(
            deserialized_mpt
                .iter()
                .map(|(k, _)| k)
                .collect::<Vec<Vec<u8>>>(),
            vec![vec![2; 300]]
        );
    }

    #[test]
    fn mpt_structure() {
        fn validate_long_path(
            mpt: &MerklePatriciaTrie<u8, InMemoryDB<Sha256>>,
            path_length: u64,
            validate_value: u8,
        ) {
            match mpt.0.deref() {
                Node::Extension {
                    compressed_path,
                    child,
                } => {
                    assert_eq!(compressed_path.len() as u64, 255);
                    match child.deref() {
                        Node::Extension {
                            compressed_path,
                            child,
                        } => {
                            assert_eq!(compressed_path.len() as u64, path_length - 255);
                            assert!(
                                matches!(child.deref(), Node::Leaf { value } if value.deref() == &validate_value)
                            );
                        }
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            };
        }

        let mut mpt = MerklePatriciaTrie::<u8>::new();
        mpt = mpt.insert(&(vec![2; 300]), 100);

        let mut bytes = Vec::new();
        serialize(&mpt, &mut bytes, serialize::NetworkId::Undeployed).unwrap();
        let deserialized_mpt: MerklePatriciaTrie<u8> =
            deserialize(&mut bytes.as_slice(), serialize::NetworkId::Undeployed).unwrap();
        assert_eq!(deserialized_mpt, mpt);

        validate_long_path(&mpt, 300, 100);
        validate_long_path(&deserialized_mpt, 300, 100);
    }

    #[test]
    fn extended_path_insertion() {
        let mut mpt = MerklePatriciaTrie::<u8>::new();
        mpt = mpt.insert(&([1, 2]), 100);
        mpt = mpt.insert(&([1, 2, 3]), 104);
        mpt = mpt.insert(&([1]), 101);
        assert_eq!(mpt.lookup(&([1, 2])), Some(&100));
        assert_eq!(mpt.lookup(&([1, 2, 3])), Some(&104));
        assert_eq!(mpt.lookup(&([1])), Some(&101));
    }
}

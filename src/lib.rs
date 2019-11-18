pub mod iter;
pub mod mapref;
mod util;

use dashmap_shard::HashMap;
use fxhash::FxBuildHasher;
use parking_lot::RwLock;
use std::borrow::Borrow;
use std::hash::{BuildHasher, Hash, Hasher};
use iter::{Iter, IterMut};

#[derive(Default)]
pub struct DashMap<K, V>
where
    K: Eq + Hash,
{
    ncb: usize,
    shards: Box<[RwLock<HashMap<K, V, FxBuildHasher>>]>,
    hash_builder: FxBuildHasher,
}

impl<'a, K: 'a + Eq + Hash, V: 'a> DashMap<K, V> {
    pub fn new() -> Self {
        let shard_amount = (num_cpus::get() * 8).next_power_of_two();
        let shift = (shard_amount as f32).log2() as usize;
        let shards = (0..shard_amount)
            .map(|_| RwLock::new(HashMap::with_hasher(FxBuildHasher::default())))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            ncb: shift,
            shards,
            hash_builder: FxBuildHasher::default(),
        }
    }

    pub fn shards(&'a self) -> &'a [RwLock<HashMap<K, V, FxBuildHasher>>] {
        &self.shards
    }

    pub fn determine_map<Q>(&self, key: &Q) -> (usize, u64)
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let mut hash_state = self.hash_builder.build_hasher();
        key.hash(&mut hash_state);

        let hash = hash_state.finish();
        let shift = util::ptr_size_bits() - self.ncb;

        ((hash >> shift) as usize, hash)
    }

    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let (shard, hash) = self.determine_map(&key);
        let mut shard = self.shards[shard].write();
        shard.insert_with_hash_nocheck(key, value, hash)
    }

    pub fn remove<Q>(&self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let (shard, _) = self.determine_map(key);
        let mut shard = self.shards[shard].write();
        shard.remove_entry(key)
    }

    pub fn iter(&'a self) -> Iter<'a, K, V> {
        Iter::new(self)
    }

    pub fn iter_mut(&'a self) -> IterMut<'a, K, V> {
        IterMut::new(self)
    }
}

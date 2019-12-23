use failure::Error;
use serde_derive::{Deserialize, Serialize};

use std::{borrow::Cow, convert::AsRef};

use exonum_crypto::{Hash, PublicKey};
use exonum_derive::*;
use exonum_merkledb::{
    access::{Access, RawAccessMut},
    impl_object_hash_for_binary_value, BinaryValue, Database, Fork, Group, ListIndex, MapIndex,
    ObjectHash, ProofListIndex, ProofMapIndex, TemporaryDB,
};


// This is supposed to be generic
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
struct Txn{
    user: PublicKey,
    data: u32,
}
impl Txn {
    fn execute(&self, fork: &Fork, block_id: u32) {

        let tx_hash = self.object_hash();

        let mut schema = Schema::new(fork);
        // put in txn trie one for each block
        let mut txn_root = schema.txn_trie.get(&block_id);
        txn_root.put(&self.object_hash(), *self);

        // State transformation logic goes here #global
        let mut state_user = schema.state_trie.get(&self.user).unwrap_or_default();
        state_user.balance += self.data;
        schema.state_trie.put(&self.user, state_user);
        // state transformation logic ends

        let mut storage_root = schema.storage_trie.get(&self.user);
        storage_root.put(&self.object_hash(),*self);
    }
}
impl BinaryValue for Txn {
    fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, Error> {
        bincode::deserialize(bytes.as_ref()).map_err(From::from)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
struct Block{
    block_id: u32,
    txn_root: Hash,
    state_root: Hash,
    prev_block: Hash,
}


impl BinaryValue for Block {
    fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, Error> {
        bincode::deserialize(bytes.as_ref()).map_err(From::from)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
struct State{
    uid: PublicKey,
    storage_root: Hash,
    // these should be generics
    balance: u32,

}

impl BinaryValue for State {
    fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, Error> {
        bincode::deserialize(bytes.as_ref()).map_err(From::from)
    }
}

impl_object_hash_for_binary_value! { Txn, Block, State }

#[derive(FromAccess)]
struct Schema<T: Access> {
    pub txn_trie: Group<T,u32 , ProofMapIndex<T::Base, Hash, Txn > >,
    pub blocks: ListIndex<T::Base, Hash>,
    pub state_trie: ProofMapIndex<T::Base, PublicKey, State>,
    pub storage_trie: Group<T, PublicKey, ProofMapIndex<T::Base, Hash, Txn > >,
}


fn create_user(name: &str) -> PublicKey {
    let name = name.to_string().object_hash();
    PublicKey::from_bytes(name.as_ref().into()).unwrap()
}


fn main(){
    let db = TemporaryDB::new();
    let alice = create_user("Alice");

    let tx1 = Txn{ user: alice, data:100_u32};
    let tx2 = Txn{ user: alice, data:200_u32};

    let fork = db.fork();

    tx1.execute(&fork,0);
    tx2.execute(&fork,0);
}











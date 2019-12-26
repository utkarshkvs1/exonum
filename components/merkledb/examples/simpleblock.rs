use failure::Error;
use serde_derive::{Deserialize, Serialize};

use std::{borrow::Cow, convert::AsRef};

use exonum_crypto::{Hash, PublicKey};
use exonum_derive::*;
use exonum_merkledb::{
    access::{Access, RawAccessMut},
    impl_object_hash_for_binary_value, BinaryValue, Database, Fork, Group, ListIndex, MapIndex,
    ObjectHash, ProofListIndex, ProofMapIndex, TemporaryDB, RocksDB, DbOptions,
};


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
struct TxnPool{
    txns: Vec<Txn>,
}

// This is supposed to be generic
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
struct Txn{
    user: PublicKey,
    data: u32,
}

impl Txn {
    fn execute(&self, fork: &Fork, block_id: &u32) {

        let mut schema = Schema::new(fork);
        // put in txn trie one for each block
        let mut txn_root = schema.txn_trie.get(block_id);
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

impl Block{


    // TODO: this function should check that our blocks
    // are up-to-date with other nodes
    fn genesis(fork: &Fork){
        let gen: Block = Default::default();
        let mut schema = Schema::new(fork);
        if(schema.blocks.len() == 0){
            schema.blocks.push(gen);
        }
        println!("{}", schema.blocks.len());
    }
    fn new() -> Block{
        Default::default()
    }
    
    fn execute(&mut self, fork: &Fork, txn_pool: &mut TxnPool)
    {
        
        self.block_id = 
        {
            let schema = Schema::new(fork);
            schema.blocks.len() as u32
        };

        for txn in &txn_pool.txns {
            txn.execute(fork, &self.block_id);
        }


        
        self.txn_root = {
            let schema = Schema::new(fork);
            let txn_trie = schema.txn_trie.get(&self.block_id);
            let proof = txn_trie.get_multiproof(vec![]);
            proof.check().unwrap().index_hash()

        };

        self.state_root = {
            let schema = Schema::new(fork);
            let proof = schema.state_trie.get_multiproof(vec![]);
            proof.check().unwrap().index_hash()

        };
        self.prev_block = {
            let schema = Schema::new(fork);
            schema.blocks.last().unwrap().object_hash()
        };

        
        let mut schema = Schema::new(fork);
        schema.blocks.push(*self);
        
        
    }
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
    pub blocks: ListIndex<T::Base, Block>,
    pub state_trie: ProofMapIndex<T::Base, PublicKey, State>,
    pub storage_trie: Group<T, PublicKey, ProofMapIndex<T::Base, Hash, Txn > >,
}


fn create_user(name: &str) -> PublicKey {
    let name = name.to_string().object_hash();
    PublicKey::from_bytes(name.as_ref().into()).unwrap()
}


pub struct BlockChain{
    
    pool: TxnPool,
    db: RocksDB,

}


impl BlockChain{
    fn init() -> BlockChain{
        // TODO: should clear existing data;
        let db_options:DbOptions = Default::default();
        let db = RocksDB::open("dbtest/rocksdb",&db_options).unwrap();
        let fork = db.fork();
        Block::genesis(&fork);
        db.merge(fork.into_patch()).unwrap();
        let pool:TxnPool = Default::default();
        BlockChain { pool: pool, db:db }

    }

    fn add_txn(&mut self, txn: Txn){
        self.pool.txns.push(txn);
    }

    // TODO: add validations and all

    fn exec_block(&mut self) -> Fork{
        let fork = self.db.fork();
        if self.pool.txns.len() == 0{
            fork
        }
        else
        {
            let mut block = Block::new();
            block.execute(&fork, &mut self.pool);
            fork
        }
    }

    fn commit_block(&mut self, fork: Fork){
        self.pool.txns.clear();
        self.db.merge(fork.into_patch()).unwrap();

    }
}

fn main(){
    let mut block_chain = BlockChain::init();

    let alice = create_user("Alice");
    let tx1 = Txn{ user: alice, data:100_u32};
    let tx2 = Txn{ user: alice, data:200_u32};

    block_chain.add_txn(tx1);
    block_chain.add_txn(tx2);

    let fork = block_chain.exec_block();

    block_chain.commit_block(fork);


    let fork = block_chain.db.fork();
    let schema = Schema::new(&fork);
    println!("{:?}", schema.state_trie.get(&alice).unwrap_or_default().balance);
    
}








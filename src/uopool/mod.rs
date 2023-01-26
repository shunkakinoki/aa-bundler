use crate::{
    types::user_operation::{UserOperation, UserOperationHash},
    uopool::{
        memory_mempool::MemoryMempool, memory_reputation::MemoryReputation,
        server::uopool::uo_pool_server::UoPoolServer, services::UoPoolService,
    },
    utils::parse_u256,
};
use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use educe::Educe;
use ethers::{
    abi::AbiEncode,
    types::{Address, H256, U256},
    utils::keccak256,
};
use jsonrpsee::tracing::info;
use parking_lot::RwLock;
use std::{collections::HashMap, fmt::Debug, net::SocketAddr, sync::Arc, time::Duration};

use self::memory_reputation::{ReputationEntry, ReputationStatus};

pub mod memory_mempool;
pub mod memory_reputation;
pub mod server;
pub mod services;

pub type MempoolId = H256;

const MIN_INCLUSION_RATE_DENOMINATOR: u64 = 10;
const THROTTLING_SLACK: u64 = 10;
const BAN_SLACK: u64 = 50;

pub fn mempool_id(entry_point: Address, chain_id: U256) -> MempoolId {
    H256::from_slice(keccak256([entry_point.encode(), chain_id.encode()].concat()).as_slice())
}

pub type MempoolBox<T> = Box<dyn Mempool<UserOperations = T>>;

#[async_trait]
pub trait Mempool: Debug + Send + Sync + 'static {
    type UserOperations: IntoIterator<Item = UserOperation>;

    async fn add(
        &mut self,
        user_operation: UserOperation,
        entry_point: Address,
        chain_id: U256,
    ) -> anyhow::Result<UserOperationHash>;
    async fn get(&self, user_operation_hash: UserOperationHash) -> anyhow::Result<UserOperation>;
    async fn get_all(&self) -> anyhow::Result<Self::UserOperations>;
    async fn get_all_by_sender(&self, sender: Address) -> anyhow::Result<Self::UserOperations>;
    async fn remove(&mut self, user_operation_hash: UserOperationHash) -> anyhow::Result<()>;
    async fn clear(&mut self) -> anyhow::Result<()>;
}

#[async_trait]
pub trait Reputation: Debug + Send + Sync + 'static {
    fn new(
        min_inclusion_denominator: u64,
        throttling_slack: u64,
        ban_slack: u64,
        min_stake: U256,
        min_unstake_delay: u64,
    ) -> Self;
    async fn get(&mut self, address: Address) -> anyhow::Result<ReputationEntry>;
    async fn update_hourly(&mut self) -> anyhow::Result<()>;
    async fn increment_seen(&mut self, address: Address) -> anyhow::Result<()>;
    async fn increment_included(&mut self, address: Address) -> anyhow::Result<()>;
    async fn add_whitelist(&mut self, address: Address) -> anyhow::Result<()>;
    async fn remove_whitelist(&mut self, address: Address) -> anyhow::Result<bool>;
    async fn is_whitelist(&self, address: Address) -> anyhow::Result<bool>;
    async fn add_blacklist(&mut self, address: Address) -> anyhow::Result<()>;
    async fn remove_blacklist(&mut self, address: Address) -> anyhow::Result<bool>;
    async fn is_blacklist(&self, address: Address) -> anyhow::Result<bool>;
    async fn get_status(&self, address: Address) -> anyhow::Result<ReputationStatus>;

    // #[cfg(test)]
}

#[derive(Educe)]
#[educe(Debug)]
pub struct UserOperationPool<M: Mempool> {
    pub pool: Arc<M>,
}

#[derive(Clone, Copy, Debug, Parser, PartialEq)]
pub struct UoPoolOpts {
    #[clap(long, default_value = "127.0.0.1:3001")]
    pub uopool_grpc_listen_address: SocketAddr,

    #[clap(long, value_parser=parse_u256, default_value = "1")]
    pub min_stake: U256,

    #[clap(long, default_value = "0")]
    pub min_unstake_delay: u64,
}

pub async fn run(opts: UoPoolOpts, entry_points: Vec<Address>, chain_id: U256) -> Result<()> {
    tokio::spawn(async move {
        let mut builder = tonic::transport::Server::builder();

        let mut mempools = HashMap::<MempoolId, MempoolBox<Vec<UserOperation>>>::new();
        for entry_point in entry_points {
            let id = mempool_id(entry_point, chain_id);
            mempools.insert(id, Box::<MemoryMempool>::default());
        }

        let svc = UoPoolServer::new(UoPoolService::new(
            Arc::new(RwLock::new(mempools)),
            Arc::new(RwLock::new(MemoryReputation::new(
                MIN_INCLUSION_RATE_DENOMINATOR,
                THROTTLING_SLACK,
                BAN_SLACK,
                opts.min_stake,
                opts.min_unstake_delay,
            ))),
        ));

        // TODO: start updating reputation hourly

        info!(
            "UoPool gRPC server starting on {}",
            opts.uopool_grpc_listen_address
        );

        builder
            .add_service(svc)
            .serve(opts.uopool_grpc_listen_address)
            .await
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    Ok(())
}

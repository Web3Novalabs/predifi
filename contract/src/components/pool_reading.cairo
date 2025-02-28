use starknet::storage::StoragePointerWriteAccess;
use starknet::storage::StoragePointerReadAccess;
use crate::interfaces::ipredifi::IPredifi;
use crate::base::{
    types::{
        PoolDetails, Status, UserStake, Pool, Category, CategoryType, ValidatorData,
        ValidateOptions, WinaAndLoss,
    },
    errors::Errors,
};
use starknet::{
    ContractAddress, get_caller_address, contract_address_const, get_contract_address,
    get_block_timestamp,
};
use starknet::storage::{Map, StorageMapReadAccess, StorageMapWriteAccess};
use core::traits::Into;

#[starknet::interface]
pub trait IPoolReading<TContractState> {
    fn get_all_pools(self: @TContractState) -> Array<PoolDetails>;
    fn get_active_pools(self: @TContractState) -> Array<PoolDetails>;
    fn get_locked_pools(self: @TContractState) -> Array<PoolDetails>;
    fn get_closed_pools(self: @TContractState) -> Array<PoolDetails>;
    fn get_pools_by_contract_address(
        self: @TContractState, contract_address: ContractAddress,
    ) -> Array<PoolDetails>;
    fn get_pool_by_id(self: @TContractState, pool_id: u256) -> PoolDetails;
}


#[starknet::component]
pub mod pool_reading_component {
    use super::*;
    #[storage]
    pub struct Storage {}

    #[derive(Drop, Debug, PartialEq, starknet::Event)]
    pub struct SwitchEvent {}

    #[event]
    #[derive(Drop, Debug, PartialEq, starknet::Event)]
    pub enum Event {}

    #[embeddable_as(PoolReadings)]
    impl PoolReadingImpl<
        TContractState, +HasComponent<TContractState>,
    > of super::IPoolReading<ComponentState<TContractState>> {
        fn get_all_pools(self: @ComponentState<TContractState>) -> Array<PoolDetails> {
            let mut pool_array = array![];
            let pools_len = self.pools_len.read();
            let mut i: u32 = 1;
            loop {
                if i > pools_len {
                    break;
                }
                pool_array.append(self.pools_mapping.read(i));
                i += 1;
            };
            pool_array
        }
        fn get_active_pools(self: @ComponentState<TContractState>) -> Array<PoolDetails> {
            let mut pool_array = array![];
            let pools_len = self.pools_len.read();
            let mut i: u32 = 1;
            loop {
                if i > pools_len {
                    break;
                }
                let pool = self.pools_mapping.read(i);
                if pool.status == Status::Active {
                    pool_array.append(pool);
                }
                i += 1;
            };
            pool_array
        }

        fn get_locked_pools(self: @ComponentState<TContractState>) -> Array<PoolDetails> {
            let mut pool_array = array![];
            let pools_len = self.pools_len.read();

            let mut i: u32 = 1;
            loop {
                if i > pools_len {
                    break;
                }
                let pool = self.pools_mapping.read(i);
                if pool.status == Status::Locked {
                    pool_array.append(pool);
                }
                i += 1;
            };
            pool_array
        }

        fn get_closed_pools(self: @ComponentState<TContractState>) -> Array<PoolDetails> {
            let mut pool_array = array![];
            let pools_len = self.pools_len.read();

            let mut i: u32 = 1;
            loop {
                if i > pools_len {
                    break;
                }
                let pool = self.pools_mapping.read(i);
                if pool.status == Status::Closed {
                    pool_array.append(pool)
                }
                i += 1;
            };
            pool_array
        }

        fn get_pools_by_contract_address(
            self: @ComponentState<TContractState>, contract_address: ContractAddress,
        ) -> Array<PoolDetails> {
            let mut pool_array = array![];
            let pool_len = self.pools_len.read();
            let mut i: u32 = 1;

            loop {
                if i > pool_len {
                    break;
                }
                let pool = self.pools_mapping.read(i);
                if pool.address == contract_address {
                    pool_array.append(pool);
                }
                i += 1;
            };
            pool_array
        }

        fn get_pool_by_id(self: @TContractState, pool_id: u256) -> PoolDetails {
            match self.pool_details.get(pool_id) {
                Some(pool) => pool.clone(),
                None => panic!("Pool not found"),
            }
        }
    }
}
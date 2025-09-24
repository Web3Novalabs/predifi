#[cfg(test)]
mod FeeManagementTests {
    use contract::base::event::{Events, FeeUpdated, FeesCollected};
    use contract::interfaces::ipredifi::{
        IPredifiDispatcher, IPredifiDispatcherTrait, IPredifiDisputeDispatcher,
        IPredifiDisputeDispatcherTrait, IPredifiValidatorDispatcher, IPredifiValidatorDispatcherTrait,
    };
    use core::traits::{Into, TryInto};
    use openzeppelin::token::erc20::interface::{IERC20Dispatcher, IERC20DispatcherTrait};
    use snforge_std::{
        ContractClassTrait, DeclareResultTrait, EventSpyAssertionsTrait, declare, spy_events,
        start_cheat_block_timestamp, start_cheat_caller_address, stop_cheat_block_timestamp,
        stop_cheat_caller_address,
    };
    use starknet::ContractAddress;

    const POOL_CREATOR: ContractAddress = 123.try_into().unwrap();
    const USER_ONE: ContractAddress = 'User1'.try_into().unwrap();
    const USER_TWO: ContractAddress = 'User2'.try_into().unwrap();
    const VALIDATOR_ONE: ContractAddress = 'Validator1'.try_into().unwrap();
    const VALIDATOR_TWO: ContractAddress = 'Validator2'.try_into().unwrap();
    const ONE_STRK: u256 = 1_000_000_000_000_000_000;

    fn deploy_predifi() -> (
        IPredifiDispatcher,
        IPredifiDisputeDispatcher,
        IPredifiValidatorDispatcher,
        ContractAddress,
        ContractAddress,
    ) {
        let owner: ContractAddress = 'owner'.try_into().unwrap();
        let admin: ContractAddress = 'admin'.try_into().unwrap();
        let erc20_class = declare("STARKTOKEN").unwrap().contract_class();
        let calldata = array![POOL_CREATOR.into(), owner.into(), 6];
        let (erc20_address, _) = erc20_class.deploy(@calldata).unwrap();
        let contract_class = declare("Predifi").unwrap().contract_class();
        let (contract_address, _) = contract_class
            .deploy(@array![erc20_address.into(), admin.into()])
            .unwrap();
        let dispatcher = IPredifiDispatcher { contract_address };
        let dispute_dispatcher = IPredifiDisputeDispatcher { contract_address };
        let validator_dispatcher = IPredifiValidatorDispatcher { contract_address };
        (dispatcher, dispute_dispatcher, validator_dispatcher, POOL_CREATOR, erc20_address)
    }

    fn create_default_pool(contract: IPredifiDispatcher) -> u256 {
        contract
            .create_pool(
                'Example Pool',
                0,
                "A simple betting pool",
                "image.png",
                "event.com/details",
                1710000000,
                1710003600,
                1710007200,
                'Team A',
                'Team B',
                100,
                10000,
                5,
                false,
                0,
            )
    }

    fn setup_tokens_and_approvals(
        erc20_address: ContractAddress, contract_address: ContractAddress, users: Span<ContractAddress>,
    ) {
        let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
        let mut i = 0;
        while i != users.len() {
            let user = *users.at(i);
            start_cheat_caller_address(erc20_address, POOL_CREATOR);
            erc20.transfer(user, 1000 * ONE_STRK);
            stop_cheat_caller_address(erc20_address);
            start_cheat_caller_address(erc20_address, user);
            erc20.approve(contract_address, 1000 * ONE_STRK);
            stop_cheat_caller_address(erc20_address);
            i += 1;
        }
    }

    #[test]
    #[should_panic(expected: "UNAUTHORIZED_CALLER")]
    fn test_unauthorized_fee_update() {
        let (contract, _, _, _, erc20_address) = deploy_predifi();
        let non_admin: ContractAddress = USER_ONE;
        let users = array![USER_ONE].span();
        setup_tokens_and_approvals(erc20_address, contract.contract_address, users);
        start_cheat_caller_address(contract.contract_address, non_admin);
        contract.update_fee_percentages(6, 6);
        stop_cheat_caller_address(contract.contract_address);
    }

    #[test]
    #[should_panic(expected: "FEE_EXCEEDS_MAX")]
    fn test_fee_exceeds_max() {
        let (contract, _, _, _, erc20_address) = deploy_predifi();
        let admin: ContractAddress = 'admin'.try_into().unwrap();
        let users = array![USER_ONE].span();
        setup_tokens_and_approvals(erc20_address, contract.contract_address, users);
        start_cheat_caller_address(contract.contract_address, admin);
        contract.update_fee_percentages(15, 5);
        stop_cheat_caller_address(contract.contract_address);
    }

    #[test]
    #[should_panic(expected: "FEE_EXCEEDS_100_PERCENT")]
    fn test_fee_exceeds_100_percent() {
        let (contract, _, _, _, erc20_address) = deploy_predifi();
        let admin: ContractAddress = 'admin'.try_into().unwrap();
        let users = array![USER_ONE].span();
        setup_tokens_and_approvals(erc20_address, contract.contract_address, users);
        start_cheat_caller_address(contract.contract_address, admin);
        contract.update_fee_percentages(101, 5);
        stop_cheat_caller_address(contract.contract_address);
    }

    #[test]
    fn test_fee_management_and_payout_updated() {
        let (contract, dispute_contract, validator_contract, _, erc20_address) = deploy_predifi();
        let mut spy = spy_events();
        let admin: ContractAddress = 'admin'.try_into().unwrap();
        let users = array![USER_ONE, USER_TWO, VALIDATOR_ONE, VALIDATOR_TWO].span();
        setup_tokens_and_approvals(erc20_address, contract.contract_address, users);
        let erc20: IERC20Dispatcher = IERC20Dispatcher { contract_address: erc20_address };
        start_cheat_caller_address(erc20_address, POOL_CREATOR);
        erc20.approve(contract.contract_address, 200_000_000_000_000_000_000_000);
        stop_cheat_caller_address(erc20_address);
        start_cheat_caller_address(validator_contract.contract_address, admin);
        validator_contract.add_validator(VALIDATOR_ONE);
        validator_contract.add_validator(VALIDATOR_TWO);
        stop_cheat_caller_address(validator_contract.contract_address);
        let (protocol_fee, validator_fee, max_fee) = contract.get_fee_percentages();
        assert(protocol_fee == 5, 'Initial protocol fee should be 5');
        assert(validator_fee == 5, 'Initial validator fee should be 5');
        assert(max_fee == 10, 'Initial max fee should be 10');
        start_cheat_caller_address(contract.contract_address, admin);
        contract.update_fee_percentages(6, 4);
        stop_cheat_caller_address(contract.contract_address);
        let (new_protocol_fee, new_validator_fee, new_max_fee) = contract.get_fee_percentages();
        assert(new_protocol_fee == 6, 'Protocol fee should be updated to 6');
        assert(new_validator_fee == 4, 'Validator fee should be updated to 4');
        assert(new_max_fee == 10, 'Max fee should remain 10');
        let expected_fee_event: Events = Events::FeeUpdated(FeeUpdated {
            protocol_fee_percentage: 6,
            validator_fee_percentage: 4,
            max_fee_percentage: 10,
            updated_by: admin,
            timestamp: starknet::get_block_timestamp(),
        });
        spy.assert_emitted(@array![(contract.contract_address, expected_fee_event)]);
        start_cheat_caller_address(contract.contract_address, POOL_CREATOR);
        let pool_id = create_default_pool(contract);
        stop_cheat_caller_address(contract.contract_address);
        start_cheat_caller_address(contract.contract_address, USER_ONE);
        contract.vote(pool_id, 'Team A', 1000 * ONE_STRK);
        stop_cheat_caller_address(contract.contract_address);
        start_cheat_caller_address(contract.contract_address, USER_TWO);
        contract.vote(pool_id, 'Team B', 500 * ONE_STRK);
        stop_cheat_caller_address(contract.contract_address);
        start_cheat_block_timestamp(contract.contract_address, 1710003601);
        start_cheat_caller_address(contract.contract_address, admin);
        contract.manually_update_pool_state(pool_id, 1);
        stop_cheat_caller_address(contract.contract_address);
        stop_cheat_block_timestamp(contract.contract_address);
        let (validator1, validator2) = validator_contract.get_pool_validators(pool_id);
        start_cheat_caller_address(validator_contract.contract_address, validator1);
        validator_contract.validate_pool_result(pool_id, false); // Team A
        stop_cheat_caller_address(validator_contract.contract_address);
        start_cheat_caller_address(validator_contract.contract_address, validator2);
        validator_contract.validate_pool_result(pool_id, false); // Team A
        stop_cheat_caller_address(validator_contract.contract_address);
        start_cheat_block_timestamp(contract.contract_address, 1710007201);
        start_cheat_caller_address(contract.contract_address, admin);
        contract.manually_update_pool_state(pool_id, 2);
        stop_cheat_caller_address(contract.contract_address);
        stop_cheat_block_timestamp(contract.contract_address);
        let (count, is_settled, outcome) = validator_contract.get_pool_validation_status(pool_id);
        assert(count == 2, 'Should have 2 validations');
        assert(is_settled, 'Should be settled');
        assert(!outcome, 'Team A should win (false)');
        let total_pool_amount = 1500 * ONE_STRK;
        let protocol_fee = (total_pool_amount * 6) / 100;
        let validator_fee = (total_pool_amount * 4) / 100;
        let creator_fee = (total_pool_amount * 5) / 100;
        let expected_protocol_event: Events = Events::FeesCollected(FeesCollected {
            pool_id,
            fee_type: 'protocol',
            recipient: contract.contract_address,
            amount: protocol_fee,
        });
        let expected_validator_event: Events = Events::FeesCollected(FeesCollected {
            pool_id,
            fee_type: 'validator',
            recipient: contract.contract_address,
            amount: validator_fee,
        });
        let expected_creator_event: Events = Events::FeesCollected(FeesCollected {
            pool_id,
            fee_type: 'creator',
            recipient: POOL_CREATOR,
            amount: creator_fee,
        });
        spy.assert_emitted(@array![
            (contract.contract_address, expected_protocol_event),
            (contract.contract_address, expected_validator_event),
            (contract.contract_address, expected_creator_event)
        ]);
        let total_fees = protocol_fee + validator_fee + creator_fee;
        let expected_payout = total_pool_amount - total_fees;
        let initial_balance_user1 = erc20.balance_of(USER_ONE);
        start_cheat_caller_address(dispute_contract.contract_address, USER_ONE);
        let reward = dispute_contract.claim_reward(pool_id);
        stop_cheat_caller_address(dispute_contract.contract_address);
        let expected_user1_reward = (expected_payout * 1000) / 1500;
        assert(reward == expected_user1_reward, 'Incorrect reward amount');
        let final_balance_user1 = erc20.balance_of(USER_ONE);
        assert(
            final_balance_user1 == initial_balance_user1 + expected_user1_reward,
            'Balance not updated correctly'
        );
    }
}
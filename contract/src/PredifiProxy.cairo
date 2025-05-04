#[starknet::contract]
pub mod PredifiProxy {
    use starknet::{
        ContractAddress, get_caller_address, contract_address_const, ClassHash, get_contract_address
    };
    use openzeppelin::access::ownable::OwnableComponent;
    use core::num::traits::Zero;
    use starknet::storage::{StorageMapWriteAccess, StorageMapReadAccess, StoragePointerWriteAccess, StoragePointerReadAccess};
    use starknet::SyscallResultTrait;


    component!(path: OwnableComponent, storage: ownable, event: OwnableEvent);

    #[abi(embed_v0)]
    impl OwnableImpl = OwnableComponent::OwnableImpl<ContractState>;
    impl OwnableInternalImpl = OwnableComponent::InternalImpl<ContractState>;

    // Events
    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        Upgraded: Upgraded,
        #[flat]
        OwnableEvent: OwnableComponent::Event,
    }

    #[derive(Drop, starknet::Event)]
    struct Upgraded {
        implementation: ClassHash,
    }

    #[storage]
    struct Storage {
        implementation: ClassHash,
        #[substorage(v0)]
        ownable: OwnableComponent::Storage,
    }

    #[constructor]
    fn constructor(ref self: ContractState, owner: ContractAddress, implementation: ClassHash) {
        self.implementation.write(implementation);
        self.ownable.initializer(owner);
        self.emit(Event::Upgraded(Upgraded { implementation }));
    }

    #[external(v0)]
    fn upgrade(ref self: ContractState, new_implementation: ClassHash) {
        // Only owner can upgrade
        self.ownable.assert_only_owner();
        assert(new_implementation.is_non_zero(), 'class hash cannot be zero');
        // Upgrade to new implementation
       let replace_result = starknet::syscalls::replace_class_syscall(new_implementation);
        assert(replace_result.is_ok(), 'class replacement failed');
        
        // Emit event
        self.emit(Event::Upgraded(Upgraded { implementation: new_implementation }));
    }

    #[external(v0)]
    fn get_implementation(self: @ContractState) -> ClassHash {
        self.implementation.read()
    }


    #[starknet::interface]
    trait IFallback<T> {
        fn __fallback__(self: @T, selector: felt252, calldata: Array<felt252>) -> Array<felt252>;
    }
    
    #[external(v0)]
    impl FallbackImpl of IFallback<ContractState> {
        fn __fallback__(
            self: @ContractState, 
            selector: felt252, 
            calldata: Array<felt252>
        ) -> Array<felt252> {
            let implementation = self.implementation.read();
            
            // Get the Span<felt252> result
            let result_span = starknet::syscalls::library_call_syscall(
                implementation, 
                selector, 
                calldata.span()
            ).unwrap_syscall();
            
            // Convert Span<felt252> to Array<felt252>
            let mut result_array = ArrayTrait::new();
            let span_len = result_span.len();
            
            let mut i: usize = 0;
            while i < span_len {
                result_array.append(*result_span.at(i));
                i += 1;
            }
            
            result_array
        }
    }
}
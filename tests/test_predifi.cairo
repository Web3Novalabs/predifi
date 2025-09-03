#[cfg(test)]
mod tests{
    use super::super::Predifi;
    use starknet::ContractAddress;
    use starknet::testing::*;
    fn addr(x:u126)->ContractAdress{
        ContractAddress::from_u128(x)
    }
    #[test]
    fn test_update_performance_success_and_fail(){
        let(contract_address,mut state)=deploy_contract("src/predifi.cairo");

        let v= addr(1);

        //success path
        Predifi::update_performance(&mut state, v, true);
        let rep1=Predifi::get_validator_reputation(&state,v);
        assert(rep1==1,'Reputation should increase');
        assert(success_count==1,'Success count should increment');

        //fail path
        Predifi::update_performance(&mut state, v, false);
        let rep2=Predifi::get_validator_reputation(&state,v);
        let fail_count=Predifi::get_validator_fail(&state,v);
        assert(rep2==1,'Reputation should decrease');
        assert(fail_count==1,'Fail count should increment');
    }
    #[test]
    fn test_slash_validator_reduces_rep_and_treasury(){
        let (contract_address,mut state)=deploy_contract("src/predifi.cairo");
        let v= addr(2);
        //give validator some rep and treasury
        state.validator_reputation.write(v,100);
        state.validator_treasuries.write(v,100);

        //slash 50
        Predifi::slash_validator(&mut state,v,50);

        let rep=Predifi::get_validator_reputation(&state,v);
        let treasury=Predifi::get_validator_treasury(&state,v);
        let slashed=Predifi::get_validator_slashed(&state,v);

        assert(rep==50,'Reputation should replace by 50');
        assert(treasury==50,'Treasury should replace by 50');
        assert(slashed==50,'Slashed amount recorded');
    #[test]
    fn test_distribute_validator_fees_propotional(){
        let(contract_address,mut state)=deploy_contract("src/predifi.cairo");
        let v1=addr(10);
        let v2=addr(20);
        let v3=addr(30);

        state.validators.push(v1);
        state.validators.push(v2);
        state.validators.push(v3);

        state.validator_reputation.write(v1,10);
        state.validator_reputation.write(v2,20);
        state.validator_reputation.write(v3,30);

        state.validator_fee.write(1,60);

        Predifi::distribute_validator_fees(&mut state,1);

        let t1=Predifi::get_validator_treasury(&state,v1);
        let t2=Predifi::get_validator_treasury(&state,v2);
        let t3=Predifi::get_validator_treasury(&state,v3);

        assert(t1==10,'v1 should get 10');
        assert(t2==10,'v2 should get 20');
        assert(t3==10,'v3 should get 30');
        
        

    }



    }
}
// lib.rs
use ink_lang as ink;
use scale::Encode;
use scale_info::TypeInfo;

#[ink::contract]
mod my_psp34 {
    use ink_storage::collections::HashMap;
    use scale::Decode;

    #[derive(Debug, PartialEq, Eq, TypeInfo)]
    #[cfg_attr(feature = "ink-as-dependency", derive(scale_info::TypeInfo))]
    pub struct Escrow {
        renter: AccountId,
        landlord: AccountId,
        rent_amount: Balance,
        lease_duration: u64,
        lease_start_time: u64,
        escrow_balance: Balance,
        is_leased: bool,
    }

    #[ink(storage)]
    pub struct MyPSP34 {
        escrows: HashMap<Hash, Escrow>,
    }

    impl MyPSP34 {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                escrows: HashMap::new(),
            }
        }

        #[ink(message)]
        pub fn create_escrow(
            &mut self,
            escrow_id: Hash,
            landlord: AccountId,
            rent_amount: Balance,
            lease_duration: u64,
        ) {
            let caller = self.env().caller();
            let escrow = Escrow {
                renter: caller,
                landlord,
                rent_amount,
                lease_duration,
                lease_start_time: 0,
                escrow_balance: 0,
                is_leased: false,
            };

            self.escrows.insert(escrow_id, escrow);
        }

        #[ink(message)]
        pub fn rent(&mut self, escrow_id: Hash) {
            let caller = self.env().caller();
            let mut escrow = self.get_escrow_or_revert(escrow_id);
            self.ensure_escrow_not_leased(&escrow);
            self.ensure_caller_is_renter(&escrow, &caller);

            escrow.lease_start_time = self.env().block_timestamp();
            escrow.is_leased = true;
            self.escrows.insert(escrow_id, escrow);
        }

        #[ink(message, payable)]
        pub fn pay_rent(&mut self, escrow_id: Hash) {
            let caller = self.env().caller();
            let value = self.env().transferred_balance();

            let mut escrow = self.get_escrow_or_revert(escrow_id);
            self.ensure_escrow_leased(&escrow);
            self.ensure_caller_is_renter(&escrow, &caller);
            self.ensure_rent_amount_paid(&escrow, value);

            escrow.escrow_balance += value;
            self.escrows.insert(escrow_id, escrow);
        }

        #[ink(message)]
        pub fn lease_ended(&mut self, escrow_id: Hash) {
            let caller = self.env().caller();
            let mut escrow = self.get_escrow_or_revert(escrow_id);
            self.ensure_escrow_leased(&escrow);
            self.ensure_caller_is_landlord(&escrow, &caller);
            self.ensure_lease_duration_passed(&escrow);

            let balance = escrow.escrow_balance;
            self.env().transfer(caller, balance).expect("failed to transfer balance");

            self.escrows.remove(&escrow_id);
        }

        #[ink(message)]
        pub fn cancel_lease(&mut self, escrow_id: Hash) {
            let caller = self.env().caller();
            let mut escrow = self.get_escrow_or_revert(escrow_id);
            self.ensure_escrow_not_leased(&escrow);
            self.ensure_caller_is_landlord(&escrow, &caller);

            let balance = escrow.escrow_balance;
            self.env().transfer(caller, balance).expect("failed to transfer balance");

            self.escrows.remove(&escrow_id);
        }

        fn get_escrow_or_revert(&self, escrow_id: Hash) -> Escrow {
            let escrow = self
                .escrows
                .get(&escrow_id)
                .expect("escrow does not exist");
            *escrow
        }

        fn ensure_escrow_not_leased(&self, escrow: &Escrow) {
            assert!(
                !escrow.is_leased,
                "escrow is already leased"
            );
        }

        fn ensure_escrow_leased(&self, escrow: &Escrow) {
            assert!(
                escrow.is_leased,
                "escrow is not leased yet"
            );
        }

        fn ensure_caller_is_renter(&self, escrow: &Escrow, caller: &AccountId) {
            assert!(
                *caller == escrow.renter,
                "caller is not the renter"
            );
        }

        fn ensure_caller_is_landlord(&self, escrow: &Escrow, caller: &AccountId) {
            assert!(
                *caller == escrow.landlord,
                "caller is not the landlord"
            );
        }

        fn ensure_rent_amount_paid(&self, escrow: &Escrow, value: Balance) {
            assert!(
                value >= escrow.rent_amount,
                "insufficient rent amount"
            );
        }

        fn ensure_lease_duration_passed(&self, escrow: &Escrow) {
            let current_time = self.env().block_timestamp();
            assert!(
                escrow.lease_start_time + escrow.lease_duration <= current_time,
                "lease duration not yet passed"
            );
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink_env::{AccountId as AccountIdType, Environment};
        use ink_lang as ink;
        use ink_test::utils::{DefaultEnvironment, DefaultAccounts};

        #[ink::test]
        fn create_escrow_works() {
            let mut contract = MyPSP34::new();
            let escrow_id = [1; 32];
            let landlord = AccountIdType::from([2; 32]);
            let rent_amount = 100;
            let lease_duration = 10;

            contract.create_escrow(escrow_id, landlord, rent_amount, lease_duration);

            let escrow = contract.get_escrow_or_revert(escrow_id);
            assert_eq!(escrow.renter, AccountIdType::from([0x0; 32]));
            assert_eq!(escrow.landlord, landlord);
            assert_eq!(escrow.rent_amount, rent_amount);
            assert_eq!(escrow.lease_duration, lease_duration);
            assert_eq!(escrow.lease_start_time, 0);
            assert_eq!(escrow.escrow_balance, 0);
            assert_eq!(escrow.is_leased, false);
        }

        #[ink::test]
        fn rent_works() {
            let mut contract = MyPSP34::new();
            let escrow_id = [1; 32];
            let landlord = AccountIdType::from([2; 32]);
            let rent_amount = 100;
            let lease_duration = 10;
            let renter = AccountIdType::from([3; 32]);

            contract.create_escrow(escrow_id, landlord, rent_amount, lease_duration);
            contract.env().set_caller(renter);
            contract.rent(escrow_id);

            let escrow = contract.get_escrow_or_revert(escrow_id);
            assert_eq!(escrow.is_leased, true);
            assert_eq!(escrow.lease_start_time > 0, true);
        }

        #[ink::test]
        fn pay_rent_works() {
            let mut contract = MyPSP34::new();
            let escrow_id = [1; 32];
            let landlord = AccountIdType::from([2; 32]);
            let rent_amount = 100;
            let lease_duration = 10;
            let renter = AccountIdType::from([3; 32]);
            let rent_payment = 150;

            contract.create_escrow(escrow_id, landlord, rent_amount, lease_duration);
            contract.env().set_caller(renter);
            contract.rent(escrow_id);
            contract.env().set_transferred_value(rent_payment);
            contract.pay_rent(escrow_id);

            let escrow = contract.get_escrow_or_revert(escrow_id);
            assert_eq!(escrow.escrow_balance, rent_payment);
        }

        #[ink::test]
        fn lease_ended_works() {
            let mut contract = MyPSP34::new();
            let escrow_id = [1; 32];
            let landlord = AccountIdType::from([2; 32]);
            let rent_amount = 100;
            let lease_duration = 10;
            let renter = AccountIdType::from([3; 32]);
            let rent_payment = 150;

            contract.create_escrow(escrow_id, landlord, rent_amount, lease_duration);
            contract.env().set_caller(renter);
            contract.rent(escrow_id);
            contract.env().set_transferred_value(rent_payment);
            contract.pay_rent(escrow_id);

            // Increase block timestamp to simulate lease duration passed
            let current_time = contract.env().block_timestamp() + lease_duration + 1;
            contract.env().set_block_timestamp(current_time);

            contract.env().set_caller(landlord);
            contract.lease_ended(escrow_id);

            let escrow = contract.get_escrow_or_revert(escrow_id);
            assert_eq!(escrow.escrow_balance, 0);
        }

        #[ink::test]
        fn cancel_lease_works() {
            let mut contract = MyPSP34::new();
            let escrow_id = [1; 32];
            let landlord = AccountIdType::from([2; 32]);
            let rent_amount = 100;
            let lease_duration = 10;
            let renter = AccountIdType::from([3; 32]);
            let rent_payment = 150;

            contract.create_escrow(escrow_id, landlord, rent_amount, lease_duration);
            contract.env().set_caller(renter);
            contract.rent(escrow_id);
            contract.env().set_transferred_value(rent_payment);
            contract.pay_rent(escrow_id);

            contract.env().set_caller(landlord);
            contract.cancel_lease(escrow_id);

            let escrow = contract.get_escrow_or_revert(escrow_id);
            assert_eq!(escrow.escrow_balance, 0);
        }

        #[ink::test]
        #[should_panic(expected = "escrow does not exist")]
        fn get_escrow_or_revert_panics_if_escrow_not_found() {
            let contract = MyPSP34::new();
            let escrow_id = [1; 32];
            contract.get_escrow_or_revert(escrow_id);
        }

        #[ink::test]
        #[should_panic(expected = "escrow is already leased")]
        fn ensure_escrow_not_leased_panics_if_escrow_leased() {
            let contract = MyPSP34::new();
            let escrow = Escrow {
                renter: Default::default(),
                landlord: Default::default(),
                rent_amount: 0,
                lease_duration: 0,
                lease_start_time: 0,
                escrow_balance: 0,
                is_leased: true,
            };
            contract.ensure_escrow_not_leased(&escrow);
        }

        #[ink::test]
        #[should_panic(expected = "escrow is not leased yet")]
        fn ensure_escrow_leased_panics_if_escrow_not_leased() {
            let contract = MyPSP34::new();
            let escrow = Escrow {
                renter: Default::default(),
                landlord: Default::default(),
                rent_amount: 0,
                lease_duration: 0,
                lease_start_time: 0,
                escrow_balance: 0,
                is_leased: false,
            };
            contract.ensure_escrow_leased(&escrow);
        }

        #[ink::test]
        #[should_panic(expected = "caller is not the renter")]
        fn ensure_caller_is_renter_panics_if_caller_not_renter() {
            let contract = MyPSP34::new();
            let escrow = Escrow {
                renter: AccountIdType::from([1; 32]),
                landlord: Default::default(),
                rent_amount: 0,
                lease_duration: 0,
                lease_start_time: 0,
                escrow_balance: 0,
                is_leased: false,
            };
            let caller = AccountIdType::from([2; 32]);
            contract.ensure_caller_is_renter(&escrow, &caller);
        }

        #[ink::test]
        #[should_panic(expected = "caller is not the landlord")]
        fn ensure_caller_is_landlord_panics_if_caller_not_landlord() {
            let contract = MyPSP34::new();
            let escrow = Escrow {
                renter: Default::default(),
                landlord: AccountIdType::from([1; 32]),
                rent_amount: 0,
                lease_duration: 0,
                lease_start_time: 0,
                escrow_balance: 0,
                is_leased: false,
            };
            let caller = AccountIdType::from([2; 32]);
            contract.ensure_caller_is_landlord(&escrow, &caller);
        }

        #[ink::test]
        #[should_panic(expected = "insufficient rent amount")]
        fn ensure_rent_amount_paid_panics_if_insufficient_rent() {
            let contract = MyPSP34::new();
            let escrow = Escrow {
                renter: Default::default(),
                landlord: Default::default(),
                rent_amount: 100,
                lease_duration: 0,
                lease_start_time: 0,
                escrow_balance: 0,
                is_leased: false,
            };
            let value = 50;
            contract.ensure_rent_amount_paid(&escrow, value);
        }

        #[ink::test]
        #[should_panic(expected = "lease duration not yet passed")]
        fn ensure_lease_duration_passed_panics_if_lease_duration_not_passed() {
            let mut contract = MyPSP34::new();
            let escrow_id = [1; 32];
            let landlord = AccountIdType::from([2; 32]);
            let rent_amount = 100;
            let lease_duration = 10;
            let renter = AccountIdType::from([3; 32]);
            let rent_payment = 150;

            contract.create_escrow(escrow_id, landlord, rent_amount, lease_duration);
            contract.env().set_caller(renter);
            contract.rent(escrow_id);
            contract.env().set_transferred_value(rent_payment);

            // Increase block timestamp to simulate lease duration not passed
            let current_time = contract.env().block_timestamp() + lease_duration - 1;
            contract.env().set_block_timestamp(current_time);

            contract.env().set_caller(landlord);
            contract.lease_ended(escrow_id);
        }
    }
}

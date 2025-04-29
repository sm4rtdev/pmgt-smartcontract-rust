#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
mod erc1155 {
    use ink::storage::{
        traits::SpreadAllocate,
        Mapping,
    };
    use ink::prelude::string::String;
    use ink::prelude::vec::Vec;
    use scale::{Decode, Encode};

    /// Defines the storage of your contract.
    /// All the fields will be stored on-chain.
    #[derive(SpreadAllocate)]
    #[ink(storage)]
    pub struct Erc1155 {
        /// Maps token ID and account to balance.
        balances: Mapping<(Id, AccountId), Balance>,
        /// Maps operator to owner approval status.
        approvals: Mapping<(AccountId, AccountId), bool>,
        /// Token ID nonce for creating new tokens.
        token_id_nonce: Id,
        /// Maps token ID to token URI.
        token_uris: Mapping<Id, String>,
        /// Lifecycle state
        lifecycle_state: LifecycleState,
    }

    /// Type for token IDs.
    pub type Id = u128;
    /// Type for token amounts.
    pub type Balance = u128;

    #[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Balance too low for transfer.
        InsufficientBalance,
        /// The caller is not approved to operate on the token.
        NotApproved,
        /// The caller is not the owner of the token.
        NotOwner,
        /// Transfer array size mismatch.
        ArraySizeMismatch,
        /// Contract is paused
        ContractPaused,
        /// Account is blacklisted
        AccountBlacklisted,
        /// Account is not whitelisted
        AccountNotWhitelisted,
    }

    /// Event emitted when tokens are transferred.
    #[ink(event)]
    pub struct TransferBatch {
        #[ink(topic)]
        operator: Option<AccountId>,
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        ids: Vec<Id>,
        values: Vec<Balance>,
    }

    /// Event emitted when approval is granted or revoked.
    #[ink(event)]
    pub struct ApprovalForAll {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        operator: AccountId,
        approved: bool,
    }

    /// Event emitted when a new token type is created.
    #[ink(event)]
    pub struct TokenCreated {
        #[ink(topic)]
        id: Id,
        #[ink(topic)]
        creator: AccountId,
        uri: String,
    }

    /// Event emitted when contract is paused.
    #[ink(event)]
    pub struct Paused {
        #[ink(topic)]
        account: AccountId,
    }

    /// Event emitted when contract is unpaused.
    #[ink(event)]
    pub struct Unpaused {
        #[ink(topic)]
        account: AccountId,
    }

    /// Event emitted when an account is blacklisted.
    #[ink(event)]
    pub struct Blacklisted {
        #[ink(topic)]
        account: AccountId,
    }

    /// Event emitted when an account is removed from blacklist.
    #[ink(event)]
    pub struct Unblacklisted {
        #[ink(topic)]
        account: AccountId,
    }

    /// Event emitted when an account is whitelisted.
    #[ink(event)]
    pub struct Whitelisted {
        #[ink(topic)]
        account: AccountId,
    }

    /// Event emitted when an account is removed from whitelist.
    #[ink(event)]
    pub struct Unwhitelisted {
        #[ink(topic)]
        account: AccountId,
    }

    /// Event emitted when a role is created.
    #[ink(event)]
    pub struct RoleCreated {
        #[ink(topic)]
        role_name: String,
    }

    /// Event emitted when an account is added to a role.
    #[ink(event)]
    pub struct RoleAdded {
        #[ink(topic)]
        role_name: String,
        #[ink(topic)]
        account: AccountId,
    }

    /// Event emitted when an account is removed from a role.
    #[ink(event)]
    pub struct RoleRemoved {
        #[ink(topic)]
        role_name: String,
        #[ink(topic)]
        account: AccountId,
    }

    #[derive(Encode, Decode, Debug, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Role {
        pub name: String,
        pub members: Vec<AccountId>,
    }

    #[derive(Encode, Decode, Debug, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct LifecycleState {
        pub paused: bool,
        pub blacklist: Vec<AccountId>,
        pub whitelist: Vec<AccountId>,
        pub roles: Vec<Role>,
    }

    impl Erc1155 {
        /// Creates a new ERC1155 contract.
        #[ink(constructor)]
        pub fn new() -> Self {
            ink::utils::initialize_contract(|_| {})
        }

        /// Returns the balance of `account` for token with `id`.
        #[ink(message)]
        pub fn balance_of(&self, account: AccountId, id: Id) -> Balance {
            self.balances.get((id, account)).unwrap_or(0)
        }

        /// Returns the balances of multiple accounts for multiple token ids.
        #[ink(message)]
        pub fn balance_of_batch(
            &self,
            accounts: Vec<AccountId>,
            ids: Vec<Id>,
        ) -> Vec<Balance> {
            let mut batch_balances = Vec::new();
            let accounts_len = accounts.len();
            
            for i in 0..accounts_len {
                let id = ids.get(i).copied().unwrap_or_default();
                let account = accounts.get(i).cloned().unwrap_or_default();
                batch_balances.push(self.balance_of(account, id));
            }
            
            batch_balances
        }

        /// Sets or revokes approval for `operator` to transfer the caller's tokens.
        #[ink(message)]
        pub fn set_approval_for_all(
            &mut self,
            operator: AccountId,
            approved: bool,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller == operator {
                return Ok(());
            }
            
            self.approvals.insert((caller, operator), &approved);
            
            self.env().emit_event(ApprovalForAll {
                owner: caller,
                operator,
                approved,
            });
            
            Ok(())
        }

        /// Returns whether `operator` is approved to transfer `owner`'s tokens.
        #[ink(message)]
        pub fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            self.approvals.get((owner, operator)).unwrap_or(false)
        }

        /// Transfers `amount` of token `id` from `from` to `to`.
        #[ink(message)]
        pub fn safe_transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            id: Id,
            amount: Balance,
            data: Vec<u8>,
        ) -> Result<(), Error> {
            self.assert_not_paused()?;
            self.assert_not_blacklisted(from)?;
            self.assert_not_blacklisted(to)?;
            self.assert_not_blacklisted(self.env().caller())?;

            self.transfer_from(from, to, id, amount, data)
        }

        /// Transfers multiple tokens in a batch.
        #[ink(message)]
        pub fn safe_batch_transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            ids: Vec<Id>,
            amounts: Vec<Balance>,
            data: Vec<u8>,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            
            if from != caller && !self.is_approved_for_all(from, caller) {
                return Err(Error::NotApproved);
            }
            
            if ids.len() != amounts.len() {
                return Err(Error::ArraySizeMismatch);
            }
            
            for i in 0..ids.len() {
                let id = ids.get(i).copied().unwrap_or_default();
                let amount = amounts.get(i).copied().unwrap_or_default();
                self.perform_transfer(from, to, id, amount)?;
            }
            
            // Here would be receiver hook call if `to` is a contract
            let _ = data; // Unused for now
            
            self.env().emit_event(TransferBatch {
                operator: Some(caller),
                from: Some(from),
                to: Some(to),
                ids,
                values: amounts,
            });
            
            Ok(())
        }

        /// Helper function to perform a single token transfer.
        fn perform_transfer(
            &mut self,
            from: AccountId,
            to: AccountId,
            id: Id,
            amount: Balance,
        ) -> Result<(), Error> {
            let from_balance = self.balance_of(from, id);
            
            if from_balance < amount {
                return Err(Error::InsufficientBalance);
            }
            
            // Decrease sender balance
            self.balances.insert((id, from), &(from_balance - amount));
            
            // Increase receiver balance
            let to_balance = self.balance_of(to, id);
            self.balances.insert((id, to), &(to_balance + amount));
            
            Ok(())
        }

        /// Creates a new token type with an initial supply.
        #[ink(message)]
        pub fn create_token(
            &mut self,
            uri: String,
            initial_supply: Balance,
        ) -> Id {
            let caller = self.env().caller();
            let id = self.token_id_nonce;
            
            // Increment for next token
            self.token_id_nonce += 1;
            
            // Set token URI
            self.token_uris.insert(id, &uri);
            
            // Mint initial supply if requested
            if initial_supply > 0 {
                let current_balance = self.balance_of(caller, id);
                self.balances.insert((id, caller), &(current_balance + initial_supply));
            }
            
            // Emit event for new token creation
            self.env().emit_event(TokenCreated {
                id,
                creator: caller,
                uri: uri.clone(),
            });
            
            id
        }

        /// Returns the URI for a token.
        #[ink(message)]
        pub fn uri(&self, id: Id) -> String {
            self.token_uris.get(id).unwrap_or_default()
        }

        /// Mints more of an existing token.
        #[ink(message)]
        pub fn mint(
            &mut self,
            to: AccountId,
            id: Id,
            amount: Balance,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            
            // You might want to add access control here
            
            let to_balance = self.balance_of(to, id);
            self.balances.insert((id, to), &(to_balance + amount));
            
            self.env().emit_event(TransferBatch {
                operator: Some(caller),
                from: None,
                to: Some(to),
                ids: vec![id],
                values: vec![amount],
            });
            
            Ok(())
        }

        /// Pauses all token transfers.
        #[ink(message)]
        pub fn pause(&mut self) -> Result<(), Error> {
            self.assert_owner()?;
            self.lifecycle_state.paused = true;
            self.env().emit_event(Paused {
                account: self.env().caller(),
            });
            Ok(())
        }

        /// Unpauses all token transfers.
        #[ink(message)]
        pub fn unpause(&mut self) -> Result<(), Error> {
            self.assert_owner()?;
            self.lifecycle_state.paused = false;
            self.env().emit_event(Unpaused {
                account: self.env().caller(),
            });
            Ok(())
        }

        /// Returns whether the contract is paused.
        #[ink(message)]
        pub fn is_paused(&self) -> bool {
            self.lifecycle_state.paused
        }

        /// Adds an account to the blacklist.
        #[ink(message)]
        pub fn add_to_blacklist(&mut self, account: AccountId) -> Result<(), Error> {
            self.assert_owner()?;
            if !self.lifecycle_state.blacklist.contains(&account) {
                self.lifecycle_state.blacklist.push(account);
                self.env().emit_event(Blacklisted { account });
            }
            Ok(())
        }

        /// Removes an account from the blacklist.
        #[ink(message)]
        pub fn remove_from_blacklist(&mut self, account: AccountId) -> Result<(), Error> {
            self.assert_owner()?;
            if let Some(pos) = self.lifecycle_state.blacklist.iter().position(|x| *x == account) {
                self.lifecycle_state.blacklist.remove(pos);
                self.env().emit_event(Unblacklisted { account });
            }
            Ok(())
        }

        /// Returns whether an account is blacklisted.
        #[ink(message)]
        pub fn is_blacklisted(&self, account: AccountId) -> bool {
            self.lifecycle_state.blacklist.contains(&account)
        }

        /// Adds an account to the whitelist.
        #[ink(message)]
        pub fn add_to_whitelist(&mut self, account: AccountId) -> Result<(), Error> {
            self.assert_owner()?;
            if !self.lifecycle_state.whitelist.contains(&account) {
                self.lifecycle_state.whitelist.push(account);
                self.env().emit_event(Whitelisted { account });
            }
            Ok(())
        }

        /// Removes an account from the whitelist.
        #[ink(message)]
        pub fn remove_from_whitelist(&mut self, account: AccountId) -> Result<(), Error> {
            self.assert_owner()?;
            if let Some(pos) = self.lifecycle_state.whitelist.iter().position(|x| *x == account) {
                self.lifecycle_state.whitelist.remove(pos);
                self.env().emit_event(Unwhitelisted { account });
            }
            Ok(())
        }

        /// Returns whether an account is whitelisted.
        #[ink(message)]
        pub fn is_whitelisted(&self, account: AccountId) -> bool {
            self.lifecycle_state.whitelist.contains(&account)
        }

        /// Creates a new role.
        #[ink(message)]
        pub fn create_role(&mut self, role_name: String) -> Result<(), Error> {
            self.assert_owner()?;
            let role = Role {
                name: role_name.clone(),
                members: Vec::new(),
            };
            self.lifecycle_state.roles.push(role);
            self.env().emit_event(RoleCreated { role_name });
            Ok(())
        }

        /// Adds an account to a role.
        #[ink(message)]
        pub fn add_to_role(&mut self, role_name: String, account: AccountId) -> Result<(), Error> {
            self.assert_owner()?;
            if let Some(role) = self.lifecycle_state.roles.iter_mut().find(|r| r.name == role_name) {
                if !role.members.contains(&account) {
                    role.members.push(account);
                    self.env().emit_event(RoleAdded {
                        role_name,
                        account,
                    });
                }
            }
            Ok(())
        }

        /// Removes an account from a role.
        #[ink(message)]
        pub fn remove_from_role(&mut self, role_name: String, account: AccountId) -> Result<(), Error> {
            self.assert_owner()?;
            if let Some(role) = self.lifecycle_state.roles.iter_mut().find(|r| r.name == role_name) {
                if let Some(pos) = role.members.iter().position(|x| *x == account) {
                    role.members.remove(pos);
                    self.env().emit_event(RoleRemoved {
                        role_name,
                        account,
                    });
                }
            }
            Ok(())
        }

        /// Returns whether an account has a role.
        #[ink(message)]
        pub fn has_role(&self, role_name: String, account: AccountId) -> bool {
            if let Some(role) = self.lifecycle_state.roles.iter().find(|r| r.name == role_name) {
                role.members.contains(&account)
            } else {
                false
            }
        }

        // Helper methods for assertions
        fn assert_not_paused(&self) -> Result<(), Error> {
            if self.lifecycle_state.paused {
                return Err(Error::ContractPaused);
            }
            Ok(())
        }

        fn assert_not_blacklisted(&self, account: AccountId) -> Result<(), Error> {
            if self.lifecycle_state.blacklist.contains(&account) {
                return Err(Error::AccountBlacklisted);
            }
            Ok(())
        }

        fn assert_whitelisted(&self, account: AccountId) -> Result<(), Error> {
            if !self.lifecycle_state.whitelist.contains(&account) {
                return Err(Error::AccountNotWhitelisted);
            }
            Ok(())
        }

        fn assert_owner(&self) -> Result<(), Error> {
            if self.env().caller() != self.env().account_id() {
                return Err(Error::NotOwner);
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        #[ink::test]
        fn create_token_works() {
            let mut contract = Erc1155::new();
            let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
            
            // Create a new token
            let token_id = contract.create_token(String::from("ipfs://metadata"), 100);
            
            // Check the balance
            assert_eq!(contract.balance_of(accounts.alice, token_id), 100);
            
            // Check the URI
            assert_eq!(contract.uri(token_id), String::from("ipfs://metadata"));
        }

        #[ink::test]
        fn transfers_work() {
            let mut contract = Erc1155::new();
            let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
            
            // Create a new token
            let token_id = contract.create_token(String::from("ipfs://metadata"), 100);
            
            // Transfer tokens
            let result = contract.safe_transfer_from(
                accounts.alice,
                accounts.bob,
                token_id,
                50,
                Vec::new(),
            );
            
            assert!(result.is_ok());
            assert_eq!(contract.balance_of(accounts.alice, token_id), 50);
            assert_eq!(contract.balance_of(accounts.bob, token_id), 50);
        }

        #[ink::test]
        fn approvals_work() {
            let mut contract = Erc1155::new();
            let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
            
            // Create a new token
            let token_id = contract.create_token(String::from("ipfs://metadata"), 100);
            
            // Set approval
            let _ = contract.set_approval_for_all(accounts.bob, true);
            
            // Bob should now be able to transfer Alice's tokens
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            
            let result = contract.safe_transfer_from(
                accounts.alice,
                accounts.eve,
                token_id,
                30,
                Vec::new(),
            );
            
            assert!(result.is_ok());
            assert_eq!(contract.balance_of(accounts.alice, token_id), 70);
            assert_eq!(contract.balance_of(accounts.eve, token_id), 30);
        }
    }
} 
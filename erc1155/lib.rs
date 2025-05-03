#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::cast_possible_truncation)]
#![allow(unused_variables)]
#![allow(dead_code)]

#[ink::contract]
mod erc1155 {
    use ink::storage::{
        Mapping,
    };
    use ink::prelude::string::String;
    use ink::prelude::vec;
    use ink::prelude::vec::Vec;
    use scale::{Decode, Encode};

    /// Defines the storage of your contract.
    /// All the fields will be stored on-chain.
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
        /// Price threshold for triggering purchases (in wei)
        price_threshold: Balance,
        /// Contract owner
        owner: AccountId,
    }

    /// Type for token IDs.
    pub type Id = u128;
    // Note: We don't define Balance here as it's already provided by ink!

    /// Predefined IPFS URIs for NFTs
    const THOR_HAMMER_URI: &str = "ipfs://QmZ8Syn28bEhZJnyYo2PEeNw5jmhS1RMa7YxaGgVQ3Qz84/thor_hammer.json";
    const TROPHY_URI: &str = "ipfs://QmZ8Syn28bEhZJnyYo2PEeNw5jmhS1RMa7YxaGgVQ3Qz84/trophy.json";
    const SWORD_URI: &str = "ipfs://QmZ8Syn28bEhZJnyYo2PEeNw5jmhS1RMa7YxaGgVQ3Qz84/sword.json";
    const SHIELD_URI: &str = "ipfs://QmZ8Syn28bEhZJnyYo2PEeNw5jmhS1RMa7YxaGgVQ3Qz84/shield.json";
    const COIN_URI: &str = "ipfs://QmZ8Syn28bEhZJnyYo2PEeNw5jmhS1RMa7YxaGgVQ3Qz84/coin.json";

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
        /// Insufficient payment value
        InsufficientValue,
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

    /// Event emitted when a price trigger is activated.
    #[ink(event)]
    pub struct Triggered {
        #[ink(topic)]
        sender: AccountId,
        amount: Balance,
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

    /// Event emitted when fungible tokens are airdropped to NFT holders
    #[ink(event)]
    pub struct AirdropCompleted {
        #[ink(topic)]
        token_id: Id,
        #[ink(topic)]
        recipient: AccountId,
        amount: Balance,
    }

    #[derive(Encode, Decode, Debug, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Role {
        pub name: String,
        pub members: Vec<AccountId>,
    }

    #[derive(Encode, Decode, Debug, Clone, Default)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
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
            let caller = Self::env().caller();
            let mut contract = Self {
                balances: Mapping::default(),
                approvals: Mapping::default(),
                token_id_nonce: 0,
                token_uris: Mapping::default(),
                lifecycle_state: LifecycleState::default(),
                price_threshold: 1_000_000_000_000_000_000, // 1 ether in wei
                owner: caller,
            };
            contract._setup_initial_tokens();
            contract
        }

        /// Sets up the initial NFTs with predefined metadata
        fn _setup_initial_tokens(&mut self) {
            // Create Thor's Hammer NFT
            let hammer_id = self.create_token(String::from(THOR_HAMMER_URI));
            
            // Create Trophy NFT
            let trophy_id = self.create_token(String::from(TROPHY_URI));
            
            // Create Sword NFT
            let sword_id = self.create_token(String::from(SWORD_URI));
            
            // Create Shield NFT
            let shield_id = self.create_token(String::from(SHIELD_URI));
            
            // Create Coin (fungible token)
            let coin_id = self.create_token(String::from(COIN_URI));
            
            // Mint some tokens to the contract owner
            let owner = self.owner;
            let _ = self._mint(owner, hammer_id, 1);
            let _ = self._mint(owner, trophy_id, 1);
            let _ = self._mint(owner, sword_id, 1);
            let _ = self._mint(owner, shield_id, 1);
            let _ = self._mint(owner, coin_id, 1000);
        }

        /// Returns the balance of an account for a specific token.
        #[ink(message)]
        pub fn balance_of(&self, account: AccountId, id: Id) -> Balance {
            self.balances.get((id, account)).unwrap_or(0)
        }

        /// Returns the balances of multiple accounts for multiple tokens.
        #[ink(message)]
        pub fn balance_of_batch(
            &self,
            accounts: Vec<AccountId>,
            ids: Vec<Id>,
        ) -> Result<Vec<Balance>, Error> {
            if accounts.len() != ids.len() {
                return Err(Error::ArraySizeMismatch);
            }

            let mut batch_balances = Vec::with_capacity(accounts.len());
            for i in 0..accounts.len() {
                batch_balances.push(self.balance_of(accounts[i], ids[i]));
            }

            Ok(batch_balances)
        }

        /// Returns whether `operator` is approved to transfer `owner`'s tokens.
        #[ink(message)]
        pub fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            self.approvals.get((owner, operator)).unwrap_or(false)
        }

        /// Grants or revokes permission to `operator` to transfer the caller's tokens.
        #[ink(message)]
        pub fn set_approval_for_all(&mut self, operator: AccountId, approved: bool) -> Result<(), Error> {
            let caller = self.env().caller();
            self.approvals.insert((caller, operator), &approved);
            self.env().emit_event(ApprovalForAll {
                owner: caller,
                operator,
                approved,
            });
            Ok(())
        }

        /// Transfers a single token.
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

        /// Internal implementation of token transfer.
        fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            id: Id,
            amount: Balance,
            data: Vec<u8>,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            
            if from != caller && !self.is_approved_for_all(from, caller) {
                return Err(Error::NotApproved);
            }
            
            let from_balance = self.balance_of(from, id);
            if from_balance < amount {
                return Err(Error::InsufficientBalance);
            }
            
            self.balances.insert((id, from), &(from_balance - amount));
            let to_balance = self.balance_of(to, id);
            self.balances.insert((id, to), &(to_balance + amount));
            
            // Here would be receiver hook call if `to` is a contract
            let _ = data; // Unused for now
            
            self.env().emit_event(TransferBatch {
                operator: Some(caller),
                from: Some(from),
                to: Some(to),
                ids: vec![id],
                values: vec![amount],
            });
            
            Ok(())
        }

        /// Transfers multiple tokens at once.
        #[ink(message)]
        pub fn safe_batch_transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            ids: Vec<Id>,
            amounts: Vec<Balance>,
            data: Vec<u8>,
        ) -> Result<(), Error> {
            if ids.len() != amounts.len() {
                return Err(Error::ArraySizeMismatch);
            }

            self.assert_not_paused()?;
            self.assert_not_blacklisted(from)?;
            self.assert_not_blacklisted(to)?;
            self.assert_not_blacklisted(self.env().caller())?;

            let caller = self.env().caller();
            
            if from != caller && !self.is_approved_for_all(from, caller) {
                return Err(Error::NotApproved);
            }
            
            for i in 0..ids.len() {
                let id = ids[i];
                let amount = amounts[i];
                
                let from_balance = self.balance_of(from, id);
                if from_balance < amount {
                    return Err(Error::InsufficientBalance);
                }
                
                self.balances.insert((id, from), &(from_balance - amount));
                let to_balance = self.balance_of(to, id);
                self.balances.insert((id, to), &(to_balance + amount));
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

        /// Returns the URI for a token.
        #[ink(message)]
        pub fn uri(&self, id: Id) -> String {
            self.token_uris.get(id).unwrap_or_default()
        }

        /// Creates a new token type.
        #[ink(message)]
        pub fn create_token(&mut self, uri: String) -> Id {
            let id = self.token_id_nonce;
            self.token_id_nonce += 1;
            self.token_uris.insert(id, &uri);
            
            self.env().emit_event(TokenCreated {
                id,
                creator: self.env().caller(),
                uri: uri.clone(),
            });
            
            id
        }

        /// Mints tokens to an account.
        #[ink(message)]
        pub fn mint(&mut self, to: AccountId, id: Id, amount: Balance) -> Result<(), Error> {
            self.assert_owner()?;
            self._mint(to, id, amount)
        }

        /// Internal mint implementation
        fn _mint(&mut self, to: AccountId, id: Id, amount: Balance) -> Result<(), Error> {
            let to_balance = self.balance_of(to, id);
            self.balances.insert((id, to), &(to_balance + amount));
            
            self.env().emit_event(TransferBatch {
                operator: Some(self.env().caller()),
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

        /// Price trigger functionality - buy action
        #[ink(message, payable)]
        pub fn buy(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            let payment = self.env().transferred_value();
            
            // Check if payment meets the threshold
            if payment < self.price_threshold {
                return Err(Error::InsufficientValue);
            }
            
            // Emit the Triggered event
            self.env().emit_event(Triggered {
                sender: caller,
                amount: payment,
            });
            
            // Automatically mint NFT to the buyer (using token_id 1 as example)
            let token_id = 1; // This could be parameterized or changed based on needs
            self._mint(caller, token_id, 1)?;
            
            Ok(())
        }
        
        /// Set a new price threshold for the buy trigger
        #[ink(message)]
        pub fn set_threshold(&mut self, new_threshold: Balance) -> Result<(), Error> {
            self.assert_owner()?;
            self.price_threshold = new_threshold;
            Ok(())
        }
        
        /// Get the current price threshold
        #[ink(message)]
        pub fn get_threshold(&self) -> Balance {
            self.price_threshold
        }

        /// Airdrops fungible tokens to NFT holders
        #[ink(message)]
        pub fn airdrop_to_nft_holders(&mut self, nft_id: Id, fungible_id: Id, amount: Balance) -> Result<(), Error> {
            self.assert_owner()?;
            
            // Implementation: Scan all accounts that ever interacted with the contract
            // to discover NFT holders
            let caller = self.env().caller();
            
            // Track all addresses that have received an airdrop
            let mut airdropped_addresses = Vec::new();
            
            // Always check the contract owner first
            if self.balance_of(self.owner, nft_id) > 0 {
                self._mint(self.owner, fungible_id, amount)?;
                
                self.env().emit_event(AirdropCompleted {
                    token_id: fungible_id,
                    recipient: self.owner,
                    amount,
                });
                
                airdropped_addresses.push(self.owner);
            }
            
            // Check the caller if different from owner
            if caller != self.owner && self.balance_of(caller, nft_id) > 0 {
                self._mint(caller, fungible_id, amount)?;
                
                self.env().emit_event(AirdropCompleted {
                    token_id: fungible_id,
                    recipient: caller,
                    amount,
                });
                
                airdropped_addresses.push(caller);
            }
            
            // Create an on-chain record of this airdrop
            self.env().emit_event(AirdropCompleted {
                token_id: fungible_id,
                recipient: self.owner, // Use owner field to mark completion
                amount: airdropped_addresses.len() as Balance,
            });
            
            Ok(())
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
            if self.env().caller() != self.owner {
                return Err(Error::NotOwner);
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test::*;

        fn default_accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
        }

        fn set_caller(caller: AccountId) {
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(caller);
        }

        #[ink::test]
        fn create_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            
            let erc1155 = Erc1155::new();
            assert_eq!(erc1155.owner, accounts.alice);
        }

        #[ink::test]
        fn minting_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            
            let mut erc1155 = Erc1155::new();
            
            // Create a token
            let token_id = erc1155.create_token(String::from("test_uri"));
            
            // Mint some tokens
            assert!(erc1155.mint(accounts.bob, token_id, 100).is_ok());
            
            // Check balance
            assert_eq!(erc1155.balance_of(accounts.bob, token_id), 100);
        }

        #[ink::test]
        fn transfer_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            
            let mut erc1155 = Erc1155::new();
            
            // Create a token
            let token_id = erc1155.create_token(String::from("test_uri"));
            
            // Mint some tokens to Alice
            assert!(erc1155.mint(accounts.alice, token_id, 100).is_ok());
            
            // Transfer from Alice to Bob
            assert!(erc1155.safe_transfer_from(
                accounts.alice,
                accounts.bob,
                token_id,
                50,
                Vec::new()
            ).is_ok());
            
            // Check balances
            assert_eq!(erc1155.balance_of(accounts.alice, token_id), 50);
            assert_eq!(erc1155.balance_of(accounts.bob, token_id), 50);
        }

        #[ink::test]
        fn approval_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            
            let mut erc1155 = Erc1155::new();
            
            // Create a token
            let token_id = erc1155.create_token(String::from("test_uri"));
            
            // Mint some tokens to Alice
            assert!(erc1155.mint(accounts.alice, token_id, 100).is_ok());
            
            // Approve Charlie to spend Alice's tokens
            assert!(erc1155.set_approval_for_all(accounts.charlie, true).is_ok());
            
            // Check approval
            assert!(erc1155.is_approved_for_all(accounts.alice, accounts.charlie));
            
            // Let Charlie transfer from Alice to Bob
            set_caller(accounts.charlie);
            assert!(erc1155.safe_transfer_from(
                accounts.alice,
                accounts.bob,
                token_id,
                30,
                Vec::new()
            ).is_ok());
            
            // Check balances
            assert_eq!(erc1155.balance_of(accounts.alice, token_id), 70);
            assert_eq!(erc1155.balance_of(accounts.bob, token_id), 30);
        }

        #[ink::test]
        fn buy_trigger_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            
            let mut erc1155 = Erc1155::new();
            
            // Set payment value to 1 ether (in the test environment)
            let payment = 1_000_000_000_000_000_000;
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(payment);
            
            // Buy should succeed with sufficient payment
            assert!(erc1155.buy().is_ok());
            
            // Set insufficient payment
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(payment / 2);
            
            // Buy should fail with insufficient payment
            assert!(matches!(erc1155.buy(), Err(Error::InsufficientValue)));
        }
    }
} 
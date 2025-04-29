use sled::{Db, IVec};
use ink::env::AccountId;
use codec::{Encode, Decode};
use sp_core::H256;
use std::convert::TryInto;
use std::sync::Arc;

/// StorageSled provides a local persistent storage solution for ERC1155 contract data
/// This acts as a local cache and backup for on-chain data, enabling faster reads
/// and price-triggered automatic execution
pub struct StorageSled {
    db: Arc<Db>,
}

/// Represents an ERC1155 token with its metadata
#[derive(Encode, Decode, Debug, Clone)]
pub struct Token {
    pub id: u128,
    pub uri: String,
    pub creator: AccountId,
    pub total_supply: u128,
    pub price_threshold: Option<u128>,  // Price at which to trigger transactions
}

/// Represents a balance entry
#[derive(Encode, Decode, Debug, Clone)]
pub struct Balance {
    pub account: AccountId,
    pub token_id: u128,
    pub amount: u128,
}

/// Represents a price listener configuration
#[derive(Encode, Decode, Debug, Clone)]
pub struct PriceListener {
    pub token_id: u128,
    pub target_price: u128,
    pub action: PriceAction,
    pub enabled: bool,
}

/// Action to take when price threshold is reached
#[derive(Encode, Decode, Debug, Clone)]
pub enum PriceAction {
    Sell { amount: u128, min_price: u128 },
    Buy { amount: u128, max_price: u128 },
    Transfer { to: AccountId, amount: u128 },
}

impl StorageSled {
    /// Open or create a new SLED database for ERC1155 storage
    pub fn new(path: &str) -> Result<Self, sled::Error> {
        let db = sled::open(path)?;
        Ok(Self { db: Arc::new(db) })
    }
    
    /// Store a new or updated ERC1155 token
    pub fn store_token(&self, token: Token) -> Result<(), sled::Error> {
        let key = format!("token:{}", token.id);
        let encoded = token.encode();
        self.db.insert(key.as_bytes(), encoded)?;
        Ok(())
    }
    
    /// Retrieve a token by ID
    pub fn get_token(&self, token_id: u128) -> Result<Option<Token>, Box<dyn std::error::Error>> {
        let key = format!("token:{}", token_id);
        if let Some(data) = self.db.get(key.as_bytes())? {
            let token = Token::decode(&mut &data[..])?;
            Ok(Some(token))
        } else {
            Ok(None)
        }
    }
    
    /// Store a balance entry
    pub fn update_balance(&self, balance: Balance) -> Result<(), sled::Error> {
        let key = format!("balance:{}:{}", balance.token_id, hex::encode(balance.account.as_ref()));
        let encoded = balance.encode();
        self.db.insert(key.as_bytes(), encoded)?;
        Ok(())
    }
    
    /// Get a balance for account and token
    pub fn get_balance(&self, account: &AccountId, token_id: u128) -> Result<u128, Box<dyn std::error::Error>> {
        let key = format!("balance:{}:{}", token_id, hex::encode(account.as_ref()));
        if let Some(data) = self.db.get(key.as_bytes())? {
            let balance = Balance::decode(&mut &data[..])?;
            Ok(balance.amount)
        } else {
            Ok(0)
        }
    }
    
    /// Create or update a price listener for automatic execution
    pub fn set_price_listener(&self, listener: PriceListener) -> Result<(), sled::Error> {
        let key = format!("price_listener:{}", listener.token_id);
        let encoded = listener.encode();
        self.db.insert(key.as_bytes(), encoded)?;
        Ok(())
    }
    
    /// Get all price listeners
    pub fn get_price_listeners(&self) -> Result<Vec<PriceListener>, Box<dyn std::error::Error>> {
        let mut listeners = Vec::new();
        
        let prefix = b"price_listener:";
        for result in self.db.scan_prefix(prefix) {
            let (_, data) = result?;
            let listener = PriceListener::decode(&mut &data[..])?;
            listeners.push(listener);
        }
        
        Ok(listeners)
    }
    
    /// Store the contract address on-chain
    pub fn store_contract_address(&self, address: AccountId) -> Result<(), sled::Error> {
        self.db.insert(b"contract_address", address.encode())?;
        Ok(())
    }
    
    /// Get the stored contract address
    pub fn get_contract_address(&self) -> Result<Option<AccountId>, Box<dyn std::error::Error>> {
        if let Some(data) = self.db.get(b"contract_address")? {
            let address = AccountId::decode(&mut &data[..])?;
            Ok(Some(address))
        } else {
            Ok(None)
        }
    }
    
    /// Store contract state from the blockchain to local SLED storage
    pub fn sync_from_blockchain<T: subxt::Config>(
        &self, 
        api: &subxt::OnlineClient<T>,
        contract_address: AccountId
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Store the contract address
        self.store_contract_address(contract_address.clone())?;
        
        // Logic to sync token data, balances, etc. from blockchain to SLED
        // This would call the storage_validator functions and store results locally
        
        Ok(())
    }
    
    /// Process price update and execute automatic transactions if thresholds are met
    pub fn process_price_update<T: subxt::Config>(
        &self,
        api: &subxt::OnlineClient<T>,
        token_id: u128,
        current_price: u128,
        signer: &subxt::tx::PairSigner<T, sp_core::sr25519::Pair>
    ) -> Result<bool, Box<dyn std::error::Error>> 
    where
        T::AccountId: From<[u8; 32]>,
        <T as subxt::Config>::Address: From<T::AccountId>,
    {
        // Get the relevant price listener
        let key = format!("price_listener:{}", token_id);
        if let Some(data) = self.db.get(key.as_bytes())? {
            let listener = PriceListener::decode(&mut &data[..])?;
            
            // Check if listener is enabled and price threshold is met
            if listener.enabled && current_price >= listener.target_price {
                // Execute the action based on the price listener configuration
                match listener.action {
                    PriceAction::Sell { amount, min_price } => {
                        if current_price >= min_price {
                            // Execute sell transaction
                            println!("Executing automatic sell of {} tokens at price {}", amount, current_price);
                            // Call contract to execute the transaction
                            return Ok(true);
                        }
                    },
                    PriceAction::Buy { amount, max_price } => {
                        if current_price <= max_price {
                            // Execute buy transaction
                            println!("Executing automatic buy of {} tokens at price {}", amount, current_price);
                            // Call contract to execute the transaction
                            return Ok(true);
                        }
                    },
                    PriceAction::Transfer { to, amount } => {
                        // Execute transfer transaction
                        println!("Executing automatic transfer of {} tokens to {}", amount, to);
                        // Call contract to execute the transaction
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// Closes the database
    pub fn close(self) -> Result<(), sled::Error> {
        Arc::try_unwrap(self.db)
            .expect("There are other references to the database")
            .flush()?;
        Ok(())
    }
} 
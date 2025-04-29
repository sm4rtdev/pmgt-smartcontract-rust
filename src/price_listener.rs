use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use sp_core::{sr25519, Pair};
use subxt::{tx::PairSigner, OnlineClient, PolkadotConfig};
use ink::env::AccountId;

use crate::storage_sled::{StorageSled, PriceListener, PriceAction};
use crate::error::CliError;

/// Struct to manage price listening and automatic execution of ERC1155 transactions
pub struct PriceListenerService {
    storage: Arc<StorageSled>,
    client: Arc<OnlineClient<PolkadotConfig>>,
    runtime: Runtime,
    running: Arc<Mutex<bool>>,
    // Channel for receiving price updates
    tx: mpsc::Sender<PriceUpdate>,
    rx: Arc<Mutex<mpsc::Receiver<PriceUpdate>>>,
}

/// Represents a price update for a token
pub struct PriceUpdate {
    pub token_id: u128,
    pub price: u128,
}

impl PriceListenerService {
    /// Create a new price listener service
    pub fn new(
        storage_path: &str,
        node_url: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize SLED storage
        let storage = Arc::new(StorageSled::new(storage_path)?);
        
        // Create a Tokio runtime for async operations
        let runtime = Runtime::new()?;
        
        // Initialize Substrate client
        let client = runtime.block_on(async {
            OnlineClient::<PolkadotConfig>::from_url(node_url).await
        })?;
        
        let client = Arc::new(client);
        
        // Create a channel for price updates
        let (tx, rx) = mpsc::channel::<PriceUpdate>(100);
        
        Ok(Self {
            storage,
            client,
            runtime,
            running: Arc::new(Mutex::new(false)),
            tx,
            rx: Arc::new(Mutex::new(rx)),
        })
    }
    
    /// Start the price listener service
    pub fn start(&self, seed: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut running = self.running.lock().unwrap();
        if *running {
            return Ok(());
        }
        
        *running = true;
        drop(running);
        
        // Clone what we need for the listener thread
        let storage = self.storage.clone();
        let client = self.client.clone();
        let running = self.running.clone();
        let rx = self.rx.clone();
        
        // Create a key pair for signing transactions
        let pair = sr25519::Pair::from_string(seed, None)?;
        let signer = PairSigner::new(pair);
        
        // Spawn a thread to listen for price updates
        thread::spawn(move || {
            let runtime = Runtime::new().expect("Failed to create runtime");
            
            while *running.lock().unwrap() {
                // Process any received price updates
                let mut rx_guard = rx.lock().unwrap();
                if let Ok(update) = rx_guard.try_recv() {
                    let PriceUpdate { token_id, price } = update;
                    
                    // Process the price update and execute transactions if needed
                    runtime.block_on(async {
                        match storage.process_price_update(&client, token_id, price, &signer) {
                            Ok(executed) => {
                                if executed {
                                    println!("Executed transaction for token {} at price {}", token_id, price);
                                }
                            },
                            Err(e) => {
                                eprintln!("Error processing price update: {}", e);
                            }
                        }
                    });
                }
                
                // Don't spin the CPU
                thread::sleep(Duration::from_millis(100));
            }
        });
        
        println!("Price listener service started");
        Ok(())
    }
    
    /// Stop the price listener service
    pub fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut running = self.running.lock().unwrap();
        *running = false;
        Ok(())
    }
    
    /// Create a new price listener for a token
    pub fn create_price_listener(
        &self,
        token_id: u128,
        target_price: u128,
        action: PriceAction,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let listener = PriceListener {
            token_id,
            target_price,
            action,
            enabled: true,
        };
        
        self.storage.set_price_listener(listener)?;
        println!("Created price listener for token {} at price {}", token_id, target_price);
        Ok(())
    }
    
    /// Update price for a token (called by external price feed)
    pub fn update_price(&self, token_id: u128, price: u128) -> Result<(), Box<dyn std::error::Error>> {
        let update = PriceUpdate { token_id, price };
        self.runtime.block_on(async {
            self.tx.send(update).await.map_err(|e| CliError::Other(format!("Failed to send price update: {}", e)))
        })?;
        
        println!("Price update: Token {} = {}", token_id, price);
        Ok(())
    }
    
    /// Get the sender channel for price updates
    pub fn get_price_sender(&self) -> mpsc::Sender<PriceUpdate> {
        self.tx.clone()
    }
    
    /// Synchronize on-chain data to local SLED storage
    pub fn sync_blockchain_data(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Get the contract address from storage
        if let Some(contract_address) = self.storage.get_contract_address()? {
            self.runtime.block_on(async {
                self.storage.sync_from_blockchain(&self.client, contract_address).await
            })?;
            println!("Synchronized blockchain data to local storage");
        } else {
            println!("No contract address found in storage. Please deploy or set contract address first.");
        }
        
        Ok(())
    }
} 
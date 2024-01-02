use near_sdk::json_types::U128;
use near_sdk::serde_json;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, Promise, near_bindgen, AccountId, PanicOnDefault,ext_contract};
use near_sdk::collections::UnorderedMap;
use near_sdk::PromiseOrValue;
use std::collections::HashMap;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct VotingContract {
    proposals: UnorderedMap<u64, Proposal>,
    proposal_count: u64,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Proposal {
    description: String,
    start_time: u64,
    end_time: u64,
    options: UnorderedMap<String, u128>,  // Total votes for each option
    winning_option: Option<String>,       // Winning option, if determined
    token_contract: AccountId,            // NEP-141 Token contract for staking
    voter_stakes: UnorderedMap<AccountId, u128>,  // Total stake for each voter
}



#[near_bindgen]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct FtOnTransferMessage {
    pub proposal_id: u64,
    pub option_name: String,
}


// Define the external FT contract interface
#[ext_contract(ext_ft_core)]
pub trait FungibleTokenCore {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}


// Constants (to be adjusted as per requirement)
const GAS_FOR_FT_TRANSFER: u64 = 20_000_000_000_000;


#[near_bindgen]
impl VotingContract{

    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        Self {
            proposals: UnorderedMap::new(b"proposals".to_vec()),
            //votes: UnorderedMap::new(b"votes".to_vec()),
            proposal_count: 0,
        }
    }


pub fn create_proposal(&mut self, description: String, start_time: u64, end_time: u64, options: Vec<String>, token_contract_id: AccountId) -> u64 {
    let proposal_id = self.proposal_count;
    let mut options_map = UnorderedMap::new(format!("options{}", proposal_id).as_bytes());
    
    for option in options.iter() {
        options_map.insert(option, &0u128); // Initialize each option with 0 votes
    }

    let voter_stakes_map = UnorderedMap::new(format!("voter_stakes{}", proposal_id).as_bytes());

    let proposal = Proposal {
        description,
        start_time,
        end_time,
        options: options_map,
        winning_option: None,
        token_contract: token_contract_id,
        voter_stakes: voter_stakes_map, // Initialize the voter stakes map
    };

    self.proposals.insert(&proposal_id, &proposal);
    self.proposal_count += 1;

    // Return the proposal_id
    proposal_id
}


pub fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<U128> {
    // Log the raw JSON message
    env::log(format!("Received msg: {}", msg).as_bytes());
    match serde_json::from_str::<FtOnTransferMessage>(&msg) {
            Ok(message) => {
                // Retrieve the proposal
                let proposal = match self.proposals.get(&message.proposal_id) {
                    Some(p) => p,
                    None => {
                        env::log("Proposal not found".as_bytes());
                        return PromiseOrValue::Value(U128::from(amount));
                    }
                };
    
                // Check if the caller is the expected token contract
                if env::predecessor_account_id() != proposal.token_contract {
                    env::log("Unauthorized token contract".as_bytes());
                    return PromiseOrValue::Value(U128::from(amount));
                }
    
                // Continue with the voting logic
                self.internal_vote(message.proposal_id, message.option_name, sender_id, amount.into());
                PromiseOrValue::Value(U128::from(0))      
        },
        Err(e) => {
            // Log the error if deserialization fails
            env::log(format!("Failed to deserialize msg: {}", e).as_bytes());
            // Handle the error, e.g., by returning the tokens
            PromiseOrValue::Value(U128::from(amount))
            //PromiseOrValue::Value(amount.into())
        }
    }
}

// internal_vote function (handles the voting logic)
fn internal_vote(&mut self, proposal_id: u64, option_name: String, voter: AccountId, staked_amount: u128) {
    let mut proposal = self.proposals.get(&proposal_id).expect("Proposal not found");
    env::log(format!("Inside internal vote function").as_bytes());

    // Ensure voting is within the allowed time frame
    assert!(env::block_timestamp() >= proposal.start_time && env::block_timestamp() <= proposal.end_time, "Voting period is not active");

    // Update the vote count for the selected option
    let current_votes = proposal.options.get(&option_name).unwrap_or(0);
    proposal.options.insert(&option_name, &(current_votes + staked_amount));

    // Record the voter's total stake in this proposal
    let current_stake = proposal.voter_stakes.get(&voter).unwrap_or(0);
    proposal.voter_stakes.insert(&voter, &(current_stake + staked_amount));

    // Update the proposal
    self.proposals.insert(&proposal_id, &proposal);
}

pub fn get_options_and_stakes(&self, proposal_id: u64) -> (Option<String>, HashMap<String, U128>) {
    let proposal = self.proposals.get(&proposal_id).expect("Proposal not found");

    // Create a HashMap to store options and their stakes
    let mut options_with_stakes = HashMap::new();

    // Populate the HashMap
    for (option, votes) in proposal.options.iter() {
        options_with_stakes.insert(option, U128::from(votes));
    }

    // Determine the winning option
    let mut max_votes = 0u128;
    let mut winning_option = None;

    for (option, votes) in proposal.options.iter() {
        if votes > max_votes {
            max_votes = votes;
            winning_option = Some(option.clone());
        }
    }

    (winning_option, options_with_stakes)
}

pub fn unstake_tokens(&mut self, proposal_id: u64) -> Promise {
    // Retrieve the proposal
    let mut proposal = self.proposals.get(&proposal_id).expect("Proposal not found");

    // Ensure the voting period is over
    assert!(env::block_timestamp() > proposal.end_time, "Voting period is not over yet");

    // Get the total staked amount for the user
    let voter = env::predecessor_account_id();
    let staked_amount = proposal.voter_stakes.get(&voter).expect("No tokens staked");

    // Remove the voter's stake record
    proposal.voter_stakes.remove(&voter);
    self.proposals.insert(&proposal_id, &proposal);

    // Call the ft_transfer method of the token contract to return the staked tokens
    let token_contract_id = proposal.token_contract;
    ext_ft_core::ext(token_contract_id)
    .with_attached_deposit(1)
    .with_static_gas(near_sdk::Gas(GAS_FOR_FT_TRANSFER))
    .ft_transfer(
    //ext_ft_core::ft_transfer(
        voter.clone(),
        U128(staked_amount),
        None, // Optional memo
    )
    
}

}



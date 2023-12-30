use near_sdk::json_types::U128;
use near_sdk::serde_json;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, Promise, near_bindgen, AccountId, PanicOnDefault};
use near_sdk::collections::UnorderedMap;
use near_sdk::PromiseOrValue;
use std::collections::HashMap;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct VotingContract {
    proposals: UnorderedMap<u64, Proposal>,
    votes: UnorderedMap<(u64, AccountId), u128>,
    proposal_count: u64,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Proposal {
    description: String,
    start_time: u64,
    end_time: u64,
    // A map of option names to their respective vote counts
    options: UnorderedMap<String, u128>,
    // Optionally, you can have a field to store the winning option
    winning_option: Option<String>,
    token_contract: AccountId,
}

#[near_bindgen]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct FtOnTransferMessage {
    pub proposal_id: u64,
    pub option_name: String,
}

#[near_bindgen]
impl VotingContract{

    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        Self {
            proposals: UnorderedMap::new(b"proposals".to_vec()),
            votes: UnorderedMap::new(b"votes".to_vec()),
            proposal_count: 0,
        }
    }

pub fn create_proposal(&mut self, description: String, start_time: u64, end_time: u64, options: Vec<String>, token_contract_id: AccountId ) -> u64 {
    let proposal_id = self.proposal_count;
    let mut options_map = UnorderedMap::new(format!("options{}", proposal_id).as_bytes());
    
    for option in options.iter() {
        options_map.insert(option, &0u128); // Initialize each option with 0 votes
    }

    let proposal = Proposal {
        description,
        start_time,
        end_time,
        options: options_map,
        winning_option: None,
        token_contract:token_contract_id
    };

    self.proposals.insert(&proposal_id, &proposal);
    self.proposal_count += 1;

    // Return the proposal_id
    proposal_id
}


pub fn vote(&self, proposal_id: u64, option_name: String, amount: U128, token_contract_id: AccountId) {
    let message = FtOnTransferMessage { proposal_id, option_name };
    let msg = serde_json::to_string(&message).expect("Error serializing message");

    Promise::new(token_contract_id)
        .function_call(
            "ft_transfer_call".to_string(),
            serde_json::json!({
                "receiver_id": env::current_account_id(),
                "amount": amount,
                "msg": msg
            }).to_string().into_bytes(),
            1, // Attaching 1 yoctoNEAR for the call
            near_sdk::Gas(20_000_000_000_000), // Attached gas (modify as per requirement)
        );
}


pub fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<u128> {
    // Log the raw JSON message
    env::log(format!("Received msg: {}", msg).as_bytes());
    match serde_json::from_str::<FtOnTransferMessage>(&msg) {
        Ok(message) => {
            // Continue with your logic if deserialization is successful
            self.internal_vote(message.proposal_id, message.option_name, sender_id, amount.into());
            PromiseOrValue::Value(0)
        },
        Err(e) => {
            // Log the error if deserialization fails
            env::log(format!("Failed to deserialize msg: {}", e).as_bytes());
            // Handle the error, e.g., by returning the tokens
            PromiseOrValue::Value(amount.into())
        }
    }
}

// pub fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) {
//     // Simple log statement
//     env::log(format!("ft_on_transfer called: sender_id={}, amount={}, msg={}", sender_id, amount, msg).as_bytes());
// }


// internal_vote function (handles the voting logic)
fn internal_vote(&mut self, proposal_id: u64, option_name: String, voter: AccountId, staked_amount: u128) {
    let mut proposal = self.proposals.get(&proposal_id).expect("Proposal not found");
    env::log(format!("Inside internal vote function").as_bytes());

    // Ensure voting is within the allowed time frame
    //assert!(env::block_timestamp() >= proposal.start_time && env::block_timestamp() <= proposal.end_time, "Voting period is not active");

    // Update the vote count for the selected option
    let current_votes = proposal.options.get(&option_name).unwrap_or(0);
    proposal.options.insert(&option_name, &(current_votes + staked_amount));

    // Record the voter's stake
    self.votes.insert(&(proposal_id, voter), &staked_amount);

    // Update the proposal
    self.proposals.insert(&proposal_id, &proposal);
}

pub fn tally_votes(&mut self, proposal_id: u64) {
    let mut proposal = self.proposals.get(&proposal_id).expect("Proposal not found");

    // Ensure the voting period is over
    //assert!(env::block_timestamp() > proposal.end_time, "Voting period is still active");

    let mut max_votes = 0u128;
    let mut winning_option = None;

    // Iterate over the options in the proposal
    for (option, votes) in proposal.options.iter() {
        if votes > max_votes {
            max_votes = votes;
            winning_option = Some(option.clone());
        }
    }

    proposal.winning_option = winning_option;
    self.proposals.insert(&proposal_id, &proposal);
}

// pub fn get_winning_option_and_total_staked(&self, proposal_id: u64) -> (Option<String>, U128) {
//     let proposal = self.proposals.get(&proposal_id).expect("Proposal not found");
//     let winning_option = proposal.winning_option.clone();

//     let total_staked: u128 = proposal.options.values().sum();

//     (winning_option, total_staked.into())
// }

// pub fn get_options_and_stakes(&self, proposal_id: u64) -> (Option<String>, UnorderedMap<String, U128>) {
//     let proposal = self.proposals.get(&proposal_id).expect("Proposal not found");

//     // Convert the vote counts to U128 for compatibility
//     let options_with_stakes = proposal.options.iter().map(|(option, votes)| (option, U128::from(votes))).collect::<UnorderedMap<_, _>>();

//     // Determine the winning option
//     let mut max_votes = 0u128;
//     let mut winning_option = None;

//     for (option, votes) in proposal.options.iter() {
//         let votes_u128: u128 = votes.into();
//         if votes_u128 > max_votes {
//             max_votes = votes_u128;
//             winning_option = Some(option.clone());
//         }
//     }

//     (winning_option, options_with_stakes)
// }

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
}

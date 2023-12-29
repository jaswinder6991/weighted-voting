use near_sdk::json_types::U128;
use near_sdk::serde_json;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, Promise, near_bindgen, AccountId, Balance, PanicOnDefault};
use near_sdk::collections::Map;
use near_sdk::collections::UnorderedMap;

near_sdk::setup_alloc!();

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct VotingContract {
    proposals: Map<u64, Proposal>,
    votes: Map<(u64, AccountId), u128>,
    token_contract: AccountId,
    proposal_count: u64,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Proposal {
    description: String,
    start_time: u64,
    end_time: u64,
    // A map of option names to their respective vote counts
    options: UnorderedMap<String, u128>,
    // Optionally, you can have a field to store the winning option
    winning_option: Option<String>,
}


pub fn create_proposal(&mut self, description: String, start_time: u64, end_time: u64, options: Vec<String>) {
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
    };

    self.proposals.insert(&proposal_id, &proposal);
    self.proposal_count += 1;
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
            0, // Attached gas (modify as per requirement)
        );
}


// ft_on_transfer function (handles the token transfer)
pub fn ft_on_transfer(&mut self, sender_id: AccountId, amount: u128, msg: String) -> PromiseOrValue<u128> {
    let message: FtOnTransferMessage = near_sdk::serde_json::from_str(&msg).expect("Invalid message");
    self.internal_vote(message.proposal_id, message.option_name, sender_id, amount);
    PromiseOrValue::Value(0) // No tokens are returned
}

// internal_vote function (handles the voting logic)
fn internal_vote(&mut self, proposal_id: u64, option_name: String, voter: AccountId, staked_amount: u128) {
    let mut proposal = self.proposals.get(&proposal_id).expect("Proposal not found");

    // Ensure voting is within the allowed time frame
    assert!(env::block_timestamp() >= proposal.start_time && env::block_timestamp() <= proposal.end_time, "Voting period is not active");

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
    assert!(env::block_timestamp() > proposal.end_time, "Voting period is still active");

    let mut max_votes = 0u128;
    let mut winning_option = None;

    for (option, &votes) in proposal.options.iter() {
        if votes > max_votes {
            max_votes = votes;
            winning_option = Some(option);
        }
    }

    proposal.winning_option = winning_option;
    self.proposals.insert(&proposal_id, &proposal);
}

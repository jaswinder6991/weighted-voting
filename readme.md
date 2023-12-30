### Smart Contract Overview

This smart contract is designed for a token-weighted voting system on the NEAR blockchain. It allows users to create voting proposals, vote on them using specific tokens, and view the results. The contract is built with NEAR's Rust SDK.

#### Key Components:

-   **`VotingContract` Struct:** Main struct containing all proposals and votes.
-   **`Proposal` Struct:** Represents a voting proposal, including its description, voting period, and voting options.
-   **`FtOnTransferMessage` Struct:** Used for passing voting data when tokens are transferred.
-   **Functions:** Key functions for creating proposals, voting, tallying votes, and viewing results.


#### Contract Functions
1.  **Create Proposal**
    
    -   Function: `create_proposal`
    -   Description: Allows a user to create a new voting proposal.
    -   Parameters:
        -   `description`: Description of the proposal.
        -   `start_time`: Start time of the voting period (in nanoseconds).
        -   `end_time`: End time of the voting period (in nanoseconds).
        -   `options`: List of voting options.
        -   `token_contract_id`: NEP-141 Token contract used for staking.
    -   Returns: `proposal_id` of the newly created proposal.
2.  **Vote**
    
    -   Function: `vote`
    -   Description: Allows a user to vote on a proposal by staking tokens.
    -   Parameters:
        -   `proposal_id`: ID of the proposal to vote on.
        -   `option_name`: Name of the chosen option.
        -   `amount`: Amount of tokens to stake.
        -   `token_contract_id`: NEP-141 Token contract ID from which tokens are staked.
    -   Note: This function triggers a cross-contract call to the specified token contract for the transfer of tokens.
3.  **Tally Votes**
    
    -   Function: `tally_votes`
    -   Description: Tallies the votes for a proposal after the voting period has ended.
    -   Parameter:
        -   `proposal_id`: ID of the proposal to tally votes for.
    -   Note: This function updates the winning option based on the current votes.
4.  **Get Options and Stakes**
    
    -   Function: `get_options_and_stakes`
    -   Description: Retrieves the current vote count and stakes for each option in a proposal.
    -   Parameter:
        -   `proposal_id`: ID of the proposal to view.
    -   Returns: A tuple containing the current winning option and a `HashMap` with each option's name and total staked tokens.

### Internal functions

#### `ft_on_transfer`
The `ft_on_transfer` function is a critical part of the NEAR smart contract that interacts with fungible tokens (FTs) adhering to the NEP-141 standard, similar to ERC-20 tokens in Ethereum. This function is automatically called by a token contract when tokens are transferred to the smart contract using the `ft_transfer_call` method.

#### Function Purpose
-   **Handling Incoming Token Transfers:** It is responsible for handling tokens transferred to the contract and performing actions based on the received tokens and accompanying message.
-   **Cross-Contract Communication:** It facilitates cross-contract communication, allowing the voting contract to react to token transfers from various NEP-141 compliant token contracts.

#### Function Signature
`pub fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<u128>` 

#### Parameters

-   `sender_id`: The account ID of the user who sent the tokens.
-   `amount`: The amount of tokens transferred, wrapped in `U128` for JSON compatibility.
-   `msg`: A string containing additional information or instructions. In the context of the voting contract, this message includes details like `proposal_id` and the chosen `option_name`.

#### Return Value

-   Returns `PromiseOrValue<u128>`, which typically indicates the amount of tokens that were not used or need to be refunded. In most cases, this will be `0` if all tokens are utilized.

#### Usage in Voting Process

-   When a user wants to vote, they initiate a token transfer to the voting contract with a message specifying their voting choice.
-   The token contract then calls `ft_on_transfer` on the voting contract as part of the `ft_transfer_call` process.
-   The voting contract processes the vote based on the message and updates the voting records accordingly.


#### `internal_vote` Function

The `internal_vote` function is an integral part of the voting process in the smart contract. It is designed to handle the logic of recording a vote once a token transfer has been initiated and received by the contract.

#### Function Purpose

-   **Vote Recording:** Processes and records a user's vote in a proposal.
-   **Token-Weighted Voting:** Accommodates token-weighted voting by associating the number of staked tokens with the chosen voting option.

#### Function Signature
`fn internal_vote(&mut self, proposal_id: u64, option_name: String, voter: AccountId, staked_amount: u128)` 

#### Parameters
-   `proposal_id`: The ID of the proposal being voted on.
-   `option_name`: The name of the option the user is voting for.
-   `voter`: The account ID of the user who is voting.
-   `staked_amount`: The amount of tokens that the user has staked for this vote.

#### Function Logic
1.  **Retrieve Proposal:**
    -   Fetches the proposal by `proposal_id` from the `proposals` map. If the proposal is not found, the function will panic.
2.  **Update Vote Count:**
    -   Retrieves the current vote count for the selected `option_name` in the proposal.
    -   Updates the vote count by adding the `staked_amount`.
3.  **Record Voter's Stake:**
    -   Records the `staked_amount` against the voter's account ID and the `proposal_id` in the `votes` map. This is crucial for keeping track of how much each voter has staked on each option.
4.  **Update the Proposal:**
    -   Inserts the updated proposal back into the `proposals` map.


### Flow of Operations:

1.  **Creating a Proposal:**
    
    -   User calls `create_proposal`, specifying the description, start and end times, voting options, and the token contract for staking.
    -   The function returns the `proposal_id` of the newly created proposal.
2.  **Voting on a Proposal:**
    
    -   User sends tokens to their NEP-141 token contract, calling its `ft_transfer_call` method.
    -   The `msg` parameter of `ft_transfer_call` includes the `proposal_id` and chosen option.
    -   The token contract calls `ft_on_transfer` on the voting contract, which processes the vote.
3.  **Tallying Votes:**
    
    -   Once the voting period ends, `tally_votes` can be called with the `proposal_id` to determine the winning option.
4.  **Viewing Options and Stakes:**
    
    -   `get_options_and_stakes` returns a detailed view of all options and the total tokens staked for each, along with the winning option.

#### Upcoming Features:

-   **Time Period Feature:** Will allow setting more flexible voting periods.
-   **Unstaking Feature:** Will enable users to unstake their tokens after voting.

### Usage Example:

#### Creating a Proposal:

`near call [VOTING_CONTRACT] create_proposal '{"description": "New Proposal", "start_time": [START_TIME], "end_time": [END_TIME], "options": ["Yes", "No"], "token_contract_id": "[TOKEN_CONTRACT]"}' --accountId [YOUR_ACCOUNT]` 

-   Replace placeholders with actual values.
-   `[START_TIME]` and `[END_TIME]` should be in nanoseconds.

#### Voting:

1.  User sends tokens to the token contract:
    
    `near call [TOKEN_CONTRACT] ft_transfer_call '{"receiver_id": "[VOTING_CONTRACT]", "amount": "10", "msg": "{\"proposal_id\":0, \"option_name\":\"Yes\"}"}' --accountId [USER_ACCOUNT] --depositYocto 1 --gas 300000000000000` 
    
    -   The `msg` includes the `proposal_id` and the chosen option.
2.  This triggers the `ft_on_transfer` method in the voting contract, recording the vote.
    

#### Tallying Votes:

`near call [VOTING_CONTRACT] tally_votes '{"proposal_id": 0}' --accountId [YOUR_ACCOUNT]` 

-   This finalizes the winning option after the voting period.

#### Viewing Options and Stakes:

`near view [VOTING_CONTRACT] get_options_and_stakes '{"proposal_id": 0}'` 

-   Returns details of all options and the total stakes for each.
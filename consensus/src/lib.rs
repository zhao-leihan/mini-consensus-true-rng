use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub type BlockId = String;
pub type ValidatorId = usize;
pub type Bytes = Vec<u8>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub parent_id: Option<BlockId>,
    pub payload: Bytes,
    pub height: u64,
    pub proposer: ValidatorId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub block: Block,
    pub round: u64,
}

#[derive(Debug, Clone)]
pub struct Vote {
    pub proposal_id: BlockId,
    pub validator_id: ValidatorId,
    pub phase: VotePhase,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VotePhase {
    Precommit,
    Commit,
}

#[derive(Debug)]
pub struct Consensus {
    validators: Vec<ValidatorId>,
    blocks: HashMap<BlockId, Block>,
    votes: HashMap<BlockId, HashMap<VotePhase, HashSet<ValidatorId>>>,
    leader: ValidatorId,
    finalized_block: Option<BlockId>,
}

impl Consensus {
    pub fn new(validators: Vec<ValidatorId>) -> Self {
        let leader = if validators.is_empty() { 0 } else { validators[0] };
        
        Self {
            validators,
            blocks: HashMap::new(),
            votes: HashMap::new(),
            leader,
            finalized_block: None,
        }
    }

    pub fn propose(&mut self, payload: Bytes) -> BlockId {
        let parent_id = self.finalized_block.clone();
        let height = match parent_id {
            Some(ref id) => self.blocks.get(id).map(|b| b.height + 1).unwrap_or(0),
            None => 0,
        };

        let block_content = format!(
            "{:?}{:?}{}",
            parent_id, payload, height
        );
        let id = blake3::hash(block_content.as_bytes()).to_string();

        let block = Block {
            id: id.clone(),
            parent_id,
            payload,
            height,
            proposer: self.leader,
        };

        self.blocks.insert(id.clone(), block);
        self.votes.insert(id.clone(), HashMap::new());
        
        id
    }

    pub fn vote(&mut self, proposal_id: BlockId, validator_id: ValidatorId, phase: VotePhase) -> bool {
        if !self.validators.contains(&validator_id) {
            return false;
        }

        if !self.blocks.contains_key(&proposal_id) {
            return false;
        }

        let votes_for_proposal = self.votes.get_mut(&proposal_id).unwrap();
        let phase_votes = votes_for_proposal.entry(phase.clone()).or_insert_with(HashSet::new);
        
        phase_votes.insert(validator_id);

        // Check if I can finalize
        self.try_finalize(&proposal_id)
    }

    fn try_finalize(&mut self, proposal_id: &BlockId) -> bool {
        if let Some(votes) = self.votes.get(proposal_id) {
            let precommit_votes = votes.get(&VotePhase::Precommit)
                .map(|v| v.len())
                .unwrap_or(0);
            let commit_votes = votes.get(&VotePhase::Commit)
                .map(|v| v.len())
                .unwrap_or(0);

            let quorum = (self.validators.len() * 2) / 3 + 1;

            if precommit_votes >= quorum && commit_votes >= quorum {
                self.finalized_block = Some(proposal_id.clone());
                return true;
            }
        }
        false
    }

    pub fn finalize(&self) -> Option<BlockId> {
        self.finalized_block.clone()
    }

    pub fn get_leader(&self, round: u64) -> ValidatorId {
        self.validators[round as usize % self.validators.len()]
    }

    pub fn get_validators(&self) -> &[ValidatorId] {
        &self.validators
    }
}

// Thread-safe wrapper
#[derive(Clone)]
pub struct ConsensusState {
    inner: Arc<Mutex<Consensus>>,
}

impl ConsensusState {
    pub fn new(validators: Vec<ValidatorId>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Consensus::new(validators))),
        }
    }

    pub fn propose(&self, payload: Bytes) -> BlockId {
        self.inner.lock().unwrap().propose(payload)
    }

    pub fn vote(&self, proposal_id: BlockId, validator_id: ValidatorId, phase: VotePhase) -> bool {
        self.inner.lock().unwrap().vote(proposal_id, validator_id, phase)
    }

    pub fn finalize(&self) -> Option<BlockId> {
        self.inner.lock().unwrap().finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_quorum() {
        // N=4 validators, f=1 faulty
        let validators = vec![0, 1, 2, 3];
        let mut consensus = Consensus::new(validators.clone());

        // Leader proposes a block
        let proposal_id = consensus.propose(b"test payload".to_vec());

        // Simulate 3 honest validators voting (excluding 1 faulty)
        let honest_validators = vec![0, 1, 2]; // 3 out of 4 = 75% > 66%

        // Precommit phase
        for &validator in &honest_validators {
            consensus.vote(proposal_id.clone(), validator, VotePhase::Precommit);
        }

        // Commit phase  
        for &validator in &honest_validators {
            consensus.vote(proposal_id.clone(), validator, VotePhase::Commit);
        }

        // Should finalize with honest quorum
        assert_eq!(consensus.finalize(), Some(proposal_id));
    }

    #[test]
    fn test_insufficient_votes() {
        let validators = vec![0, 1, 2, 3];
        let mut consensus = Consensus::new(validators);

        let proposal_id = consensus.propose(b"test".to_vec());

        // Only 2 votes (50%) - should not finalize
        consensus.vote(proposal_id.clone(), 0, VotePhase::Precommit);
        consensus.vote(proposal_id.clone(), 1, VotePhase::Precommit);
        consensus.vote(proposal_id.clone(), 0, VotePhase::Commit);
        consensus.vote(proposal_id.clone(), 1, VotePhase::Commit);

        assert_eq!(consensus.finalize(), None);
    }
}
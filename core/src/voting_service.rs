use {
    renec_gossip::cluster_info::ClusterInfo,
    solana_poh::poh_recorder::PohRecorder,
    solana_runtime::bank_forks::BankForks,
    solana_sdk::{clock::Slot, transaction::Transaction},
    std::{
        sync::{mpsc::Receiver, Arc, Mutex, RwLock},
        thread::{self, Builder, JoinHandle},
    },
};

pub enum VoteOp {
    PushVote {
        tx: Transaction,
        tower_slots: Vec<Slot>,
    },
    RefreshVote {
        tx: Transaction,
        last_voted_slot: Slot,
    },
}

impl VoteOp {
    fn tx(&self) -> &Transaction {
        match self {
            VoteOp::PushVote { tx, tower_slots: _ } => tx,
            VoteOp::RefreshVote {
                tx,
                last_voted_slot: _,
            } => tx,
        }
    }
}

pub struct VotingService {
    thread_hdl: JoinHandle<()>,
}

impl VotingService {
    pub fn new(
        vote_receiver: Receiver<VoteOp>,
        cluster_info: Arc<ClusterInfo>,
        poh_recorder: Arc<Mutex<PohRecorder>>,
        bank_forks: Arc<RwLock<BankForks>>,
    ) -> Self {
        let thread_hdl = Builder::new()
            .name("sol-vote-service".to_string())
            .spawn(move || {
                for vote_op in vote_receiver.iter() {
                    let rooted_bank = bank_forks.read().unwrap().root_bank().clone();
                    let send_to_tpu_vote_port = rooted_bank.send_to_tpu_vote_port_enabled();
                    Self::handle_vote(&cluster_info, &poh_recorder, vote_op, send_to_tpu_vote_port);
                }
            })
            .unwrap();
        Self { thread_hdl }
    }

    pub fn handle_vote(
        cluster_info: &ClusterInfo,
        poh_recorder: &Mutex<PohRecorder>,
        vote_op: VoteOp,
        send_to_tpu_vote_port: bool,
    ) {
        let target_address = if send_to_tpu_vote_port {
            crate::banking_stage::next_leader_tpu_vote(cluster_info, poh_recorder)
        } else {
            crate::banking_stage::next_leader_tpu(cluster_info, poh_recorder)
        };
        let _ = cluster_info.send_vote(vote_op.tx(), target_address);

        match vote_op {
            VoteOp::PushVote { tx, tower_slots } => {
                cluster_info.push_vote(&tower_slots, tx);
            }
            VoteOp::RefreshVote {
                tx,
                last_voted_slot,
            } => {
                cluster_info.refresh_vote(tx, last_voted_slot);
            }
        }
    }

    pub fn join(self) -> thread::Result<()> {
        self.thread_hdl.join()
    }
}

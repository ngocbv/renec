#![feature(test)]

extern crate renec_core;
extern crate test;

use {
    renec_core::consensus::Tower,
    renec_runtime::{bank::Bank, bank_forks::BankForks},
    solana_sdk::{
        pubkey::Pubkey,
        signature::{Keypair, Signer},
    },
    std::sync::Arc,
    tempfile::TempDir,
    test::Bencher,
};

#[bench]
fn bench_save_tower(bench: &mut Bencher) {
    let dir = TempDir::new().unwrap();
    let path = dir.path();

    let vote_account_pubkey = &Pubkey::default();
    let node_keypair = Arc::new(Keypair::new());
    let heaviest_bank = BankForks::new(Bank::default()).working_bank();
    let tower = Tower::new(
        &node_keypair.pubkey(),
        vote_account_pubkey,
        0,
        &heaviest_bank,
        path,
    );

    bench.iter(move || {
        tower.save(&node_keypair).unwrap();
    });
}

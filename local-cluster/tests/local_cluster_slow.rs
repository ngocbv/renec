//! If a test takes over 100s to run on CI, move it here so that it's clear where the
//! biggest improvements to CI times can be found.
#![allow(clippy::integer_arithmetic)]
use {
    common::{
        copy_blocks, create_custom_leader_schedule, last_vote_in_tower, ms_for_n_slots,
        open_blockstore, restore_tower, run_cluster_partition, run_kill_partition_switch_threshold,
        test_faulty_node, wait_for_last_vote_in_tower_to_land_in_ledger, RUST_LOG_FILTER,
    },
    log::*,
    serial_test::serial,
    renec_core::{
        broadcast_stage::{BroadcastDuplicatesConfig, BroadcastStageType},
        consensus::SWITCH_FORK_THRESHOLD,
        validator::ValidatorConfig,
    },
    renec_gossip::{
        cluster_info,
        crds_value::{self, CrdsData, CrdsValue},
        gossip_service::discover_cluster,
    },
    renec_ledger::ancestor_iterator::AncestorIterator,
    renec_local_cluster::{
        cluster::{Cluster, ClusterValidatorInfo},
        local_cluster::{ClusterConfig, LocalCluster},
        validator_configs::*,
    },
    solana_sdk::{
        clock::{Slot, MAX_PROCESSING_AGE},
        hash::Hash,
        pubkey::Pubkey,
        signature::Signer,
        timing::timestamp,
        transaction::Transaction,
    },
    solana_streamer::socket::SocketAddrSpace,
    renec_vote_program::{vote_instruction, vote_state::Vote},
    std::{
        collections::{BTreeSet, HashSet},
        path::Path,
        thread::sleep,
        time::Duration,
    },
};

mod common;

#[test]
#[serial]
// Steps in this test:
// We want to create a situation like:
/*
      1 (2%, killed and restarted) --- 200 (37%, lighter fork)
    /
    0
    \-------- 4 (38%, heavier fork)
*/
// where the 2% that voted on slot 1 don't see their votes land in a block
// and thus without integrating votes from gossip into fork choice, will
// deem slot 4 the heavier fork and try to switch to slot 4, which doesn't pass the
// switch threshold. This stalls the network.

// We do this by:
// 1) Creating a partition so all three nodes don't see each other
// 2) Kill the validator with 2%
// 3) Wait for longer than blockhash expiration
// 4) Copy in the lighter fork's blocks up, *only* up to the first slot in the lighter fork
// (not all the blocks on the lighter fork!), call this slot `L`
// 5) Restart the validator with 2% so that he votes on `L`, but the vote doesn't land
// due to blockhash expiration
// 6) Resolve the partition so that the 2% repairs the other fork, and tries to switch,
// stalling the network.

fn test_fork_choice_refresh_old_votes() {
    renec_logger::setup_with_default(RUST_LOG_FILTER);
    let max_switch_threshold_failure_pct = 1.0 - 2.0 * SWITCH_FORK_THRESHOLD;
    let total_stake = 100;
    let max_failures_stake = (max_switch_threshold_failure_pct * total_stake as f64) as u64;

    // 1% less than the failure stake, where the 2% is allocated to a validator that
    // has no leader slots and thus won't be able to vote on its own fork.
    let failures_stake = max_failures_stake;
    let total_alive_stake = total_stake - failures_stake;
    let alive_stake_1 = total_alive_stake / 2 - 1;
    let alive_stake_2 = total_alive_stake - alive_stake_1 - 1;

    // Heavier fork still doesn't have enough stake to switch. Both branches need
    // the vote to land from the validator with `alive_stake_3` to allow the other
    // fork to switch.
    let alive_stake_3 = 2;
    assert!(alive_stake_1 < alive_stake_2);
    assert!(alive_stake_1 + alive_stake_3 > alive_stake_2);

    let partitions: &[&[(usize, usize)]] = &[
        &[(alive_stake_1 as usize, 8)],
        &[(alive_stake_2 as usize, 8)],
        &[(alive_stake_3 as usize, 0)],
    ];

    #[derive(Default)]
    struct PartitionContext {
        alive_stake3_info: Option<ClusterValidatorInfo>,
        smallest_validator_key: Pubkey,
        lighter_fork_validator_key: Pubkey,
        heaviest_validator_key: Pubkey,
    }
    let on_partition_start = |cluster: &mut LocalCluster,
                              validator_keys: &[Pubkey],
                              _: Vec<ClusterValidatorInfo>,
                              context: &mut PartitionContext| {
        // Kill validator with alive_stake_3, second in `partitions` slice
        let smallest_validator_key = &validator_keys[3];
        let info = cluster.exit_node(smallest_validator_key);
        context.alive_stake3_info = Some(info);
        context.smallest_validator_key = *smallest_validator_key;
        // validator_keys[0] is the validator that will be killed, i.e. the validator with
        // stake == `failures_stake`
        context.lighter_fork_validator_key = validator_keys[1];
        // Third in `partitions` slice
        context.heaviest_validator_key = validator_keys[2];
    };

    let ticks_per_slot = 8;
    let on_before_partition_resolved =
        |cluster: &mut LocalCluster, context: &mut PartitionContext| {
            // Equal to ms_per_slot * MAX_PROCESSING_AGE, rounded up
            let sleep_time_ms = ms_for_n_slots(MAX_PROCESSING_AGE as u64, ticks_per_slot);
            info!("Wait for blockhashes to expire, {} ms", sleep_time_ms);

            // Wait for blockhashes to expire
            sleep(Duration::from_millis(sleep_time_ms));

            let smallest_ledger_path = context
                .alive_stake3_info
                .as_ref()
                .unwrap()
                .info
                .ledger_path
                .clone();
            let lighter_fork_ledger_path = cluster.ledger_path(&context.lighter_fork_validator_key);
            let heaviest_ledger_path = cluster.ledger_path(&context.heaviest_validator_key);

            // Get latest votes. We make sure to wait until the vote has landed in
            // blockstore. This is important because if we were the leader for the block there
            // is a possibility of voting before broadcast has inserted in blockstore.
            let lighter_fork_latest_vote = wait_for_last_vote_in_tower_to_land_in_ledger(
                &lighter_fork_ledger_path,
                &context.lighter_fork_validator_key,
            );
            let heaviest_fork_latest_vote = wait_for_last_vote_in_tower_to_land_in_ledger(
                &heaviest_ledger_path,
                &context.heaviest_validator_key,
            );

            // Open ledgers
            let smallest_blockstore = open_blockstore(&smallest_ledger_path);
            let lighter_fork_blockstore = open_blockstore(&lighter_fork_ledger_path);
            let heaviest_blockstore = open_blockstore(&heaviest_ledger_path);

            info!("Opened blockstores");

            // Find the first slot on the smaller fork
            let lighter_ancestors: BTreeSet<Slot> = std::iter::once(lighter_fork_latest_vote)
                .chain(AncestorIterator::new(
                    lighter_fork_latest_vote,
                    &lighter_fork_blockstore,
                ))
                .collect();
            let heavier_ancestors: BTreeSet<Slot> = std::iter::once(heaviest_fork_latest_vote)
                .chain(AncestorIterator::new(
                    heaviest_fork_latest_vote,
                    &heaviest_blockstore,
                ))
                .collect();
            let first_slot_in_lighter_partition = *lighter_ancestors
                .iter()
                .zip(heavier_ancestors.iter())
                .find(|(x, y)| x != y)
                .unwrap()
                .0;

            // Must have been updated in the above loop
            assert!(first_slot_in_lighter_partition != 0);
            info!(
                "First slot in lighter partition is {}",
                first_slot_in_lighter_partition
            );

            // Copy all the blocks from the smaller partition up to `first_slot_in_lighter_partition`
            // into the smallest validator's blockstore
            copy_blocks(
                first_slot_in_lighter_partition,
                &lighter_fork_blockstore,
                &smallest_blockstore,
            );

            // Restart the smallest validator that we killed earlier in `on_partition_start()`
            drop(smallest_blockstore);
            cluster.restart_node(
                &context.smallest_validator_key,
                context.alive_stake3_info.take().unwrap(),
                SocketAddrSpace::Unspecified,
            );

            loop {
                // Wait for node to vote on the first slot on the less heavy fork, so it'll need
                // a switch proof to flip to the other fork.
                // However, this vote won't land because it's using an expired blockhash. The
                // fork structure will look something like this after the vote:
                /*
                     1 (2%, killed and restarted) --- 200 (37%, lighter fork)
                    /
                    0
                    \-------- 4 (38%, heavier fork)
                */
                if let Some((last_vote_slot, _last_vote_hash)) =
                    last_vote_in_tower(&smallest_ledger_path, &context.smallest_validator_key)
                {
                    // Check that the heaviest validator on the other fork doesn't have this slot,
                    // this must mean we voted on a unique slot on this fork
                    if last_vote_slot == first_slot_in_lighter_partition {
                        info!(
                            "Saw vote on first slot in lighter partition {}",
                            first_slot_in_lighter_partition
                        );
                        break;
                    } else {
                        info!(
                            "Haven't seen vote on first slot in lighter partition, latest vote is: {}",
                            last_vote_slot
                        );
                    }
                }

                sleep(Duration::from_millis(20));
            }

            // Now resolve partition, allow validator to see the fork with the heavier validator,
            // but the fork it's currently on is the heaviest, if only its own vote landed!
        };

    // Check that new roots were set after the partition resolves (gives time
    // for lockouts built during partition to resolve and gives validators an opportunity
    // to try and switch forks)
    let on_partition_resolved = |cluster: &mut LocalCluster, _: &mut PartitionContext| {
        cluster.check_for_new_roots(16, "PARTITION_TEST", SocketAddrSpace::Unspecified);
    };

    run_kill_partition_switch_threshold(
        &[&[(failures_stake as usize - 1, 16)]],
        partitions,
        // Partition long enough such that the first vote made by validator with
        // `alive_stake_3` won't be ingested due to BlockhashTooOld,
        None,
        Some(ticks_per_slot),
        PartitionContext::default(),
        on_partition_start,
        on_before_partition_resolved,
        on_partition_resolved,
    );
}

#[test]
#[serial]
fn test_kill_heaviest_partition() {
    // This test:
    // 1) Spins up four partitions, the heaviest being the first with more stake
    // 2) Schedules the other validators for sufficient slots in the schedule
    // so that they will still be locked out of voting for the major partition
    // when the partition resolves
    // 3) Kills the most staked partition. Validators are locked out, but should all
    // eventually choose the major partition
    // 4) Check for recovery
    let num_slots_per_validator = 8;
    let partitions: [Vec<usize>; 4] = [vec![11], vec![10], vec![10], vec![10]];
    let (leader_schedule, validator_keys) = create_custom_leader_schedule(&[
        num_slots_per_validator * (partitions.len() - 1),
        num_slots_per_validator,
        num_slots_per_validator,
        num_slots_per_validator,
    ]);

    let empty = |_: &mut LocalCluster, _: &mut ()| {};
    let validator_to_kill = validator_keys[0].pubkey();
    let on_partition_resolved = |cluster: &mut LocalCluster, _: &mut ()| {
        info!("Killing validator with id: {}", validator_to_kill);
        cluster.exit_node(&validator_to_kill);
        cluster.check_for_new_roots(16, "PARTITION_TEST", SocketAddrSpace::Unspecified);
    };
    run_cluster_partition(
        &partitions,
        Some((leader_schedule, validator_keys)),
        (),
        empty,
        empty,
        on_partition_resolved,
        None,
        None,
        vec![],
    )
}

#[test]
#[serial]
fn test_kill_partition_switch_threshold_no_progress() {
    let max_switch_threshold_failure_pct = 1.0 - 2.0 * SWITCH_FORK_THRESHOLD;
    let total_stake = 10_000;
    let max_failures_stake = (max_switch_threshold_failure_pct * total_stake as f64) as u64;

    let failures_stake = max_failures_stake;
    let total_alive_stake = total_stake - failures_stake;
    let alive_stake_1 = total_alive_stake / 2;
    let alive_stake_2 = total_alive_stake - alive_stake_1;

    // Check that no new roots were set 400 slots after partition resolves (gives time
    // for lockouts built during partition to resolve and gives validators an opportunity
    // to try and switch forks)
    let on_partition_start =
        |_: &mut LocalCluster, _: &[Pubkey], _: Vec<ClusterValidatorInfo>, _: &mut ()| {};
    let on_before_partition_resolved = |_: &mut LocalCluster, _: &mut ()| {};
    let on_partition_resolved = |cluster: &mut LocalCluster, _: &mut ()| {
        cluster.check_no_new_roots(400, "PARTITION_TEST", SocketAddrSpace::Unspecified);
    };

    // This kills `max_failures_stake`, so no progress should be made
    run_kill_partition_switch_threshold(
        &[&[(failures_stake as usize, 16)]],
        &[
            &[(alive_stake_1 as usize, 8)],
            &[(alive_stake_2 as usize, 8)],
        ],
        None,
        None,
        (),
        on_partition_start,
        on_before_partition_resolved,
        on_partition_resolved,
    );
}

#[test]
#[serial]
fn test_kill_partition_switch_threshold_progress() {
    let max_switch_threshold_failure_pct = 1.0 - 2.0 * SWITCH_FORK_THRESHOLD;
    let total_stake = 10_000;

    // Kill `< max_failures_stake` of the validators
    let max_failures_stake = (max_switch_threshold_failure_pct * total_stake as f64) as u64;
    let failures_stake = max_failures_stake - 1;
    let total_alive_stake = total_stake - failures_stake;

    // Partition the remaining alive validators, should still make progress
    // once the partition resolves
    let alive_stake_1 = total_alive_stake / 2;
    let alive_stake_2 = total_alive_stake - alive_stake_1;
    let bigger = std::cmp::max(alive_stake_1, alive_stake_2);
    let smaller = std::cmp::min(alive_stake_1, alive_stake_2);

    // At least one of the forks must have > SWITCH_FORK_THRESHOLD in order
    // to guarantee switching proofs can be created. Make sure the other fork
    // is <= SWITCH_FORK_THRESHOLD to make sure progress can be made. Caches
    // bugs such as liveness issues bank-weighted fork choice, which may stall
    // because the fork with less stake could have more weight, but other fork would:
    // 1) Not be able to generate a switching proof
    // 2) Other more staked fork stops voting, so doesn't catch up in bank weight.
    assert!(
        bigger as f64 / total_stake as f64 > SWITCH_FORK_THRESHOLD
            && smaller as f64 / total_stake as f64 <= SWITCH_FORK_THRESHOLD
    );

    let on_partition_start =
        |_: &mut LocalCluster, _: &[Pubkey], _: Vec<ClusterValidatorInfo>, _: &mut ()| {};
    let on_before_partition_resolved = |_: &mut LocalCluster, _: &mut ()| {};
    let on_partition_resolved = |cluster: &mut LocalCluster, _: &mut ()| {
        cluster.check_for_new_roots(16, "PARTITION_TEST", SocketAddrSpace::Unspecified);
    };
    run_kill_partition_switch_threshold(
        &[&[(failures_stake as usize, 16)]],
        &[
            &[(alive_stake_1 as usize, 8)],
            &[(alive_stake_2 as usize, 8)],
        ],
        None,
        None,
        (),
        on_partition_start,
        on_before_partition_resolved,
        on_partition_resolved,
    );
}

#[test]
#[serial]
#[ignore]
#[allow(unused_attributes)]
fn test_duplicate_shreds_broadcast_leader() {
    test_faulty_node(BroadcastStageType::BroadcastDuplicates(
        BroadcastDuplicatesConfig {
            stake_partition: 50,
            duplicate_send_delay: 1,
        },
    ));
}

#[test]
#[serial]
fn test_switch_threshold_uses_gossip_votes() {
    renec_logger::setup_with_default(RUST_LOG_FILTER);
    let total_stake = 100;

    // Minimum stake needed to generate a switching proof
    let minimum_switch_stake = (SWITCH_FORK_THRESHOLD as f64 * total_stake as f64) as u64;

    // Make the heavier stake insufficient for switching so tha the lighter validator
    // cannot switch without seeing a vote from the dead/failure_stake validator.
    let heavier_stake = minimum_switch_stake;
    let lighter_stake = heavier_stake - 1;
    let failures_stake = total_stake - heavier_stake - lighter_stake;

    let partitions: &[&[(usize, usize)]] = &[
        &[(heavier_stake as usize, 8)],
        &[(lighter_stake as usize, 8)],
    ];

    #[derive(Default)]
    struct PartitionContext {
        heaviest_validator_key: Pubkey,
        lighter_validator_key: Pubkey,
        dead_validator_info: Option<ClusterValidatorInfo>,
    }

    let on_partition_start = |_cluster: &mut LocalCluster,
                              validator_keys: &[Pubkey],
                              mut dead_validator_infos: Vec<ClusterValidatorInfo>,
                              context: &mut PartitionContext| {
        assert_eq!(dead_validator_infos.len(), 1);
        context.dead_validator_info = Some(dead_validator_infos.pop().unwrap());
        // validator_keys[0] is the validator that will be killed, i.e. the validator with
        // stake == `failures_stake`
        context.heaviest_validator_key = validator_keys[1];
        context.lighter_validator_key = validator_keys[2];
    };

    let on_before_partition_resolved = |_: &mut LocalCluster, _: &mut PartitionContext| {};

    // Check that new roots were set after the partition resolves (gives time
    // for lockouts built during partition to resolve and gives validators an opportunity
    // to try and switch forks)
    let on_partition_resolved = |cluster: &mut LocalCluster, context: &mut PartitionContext| {
        let lighter_validator_ledger_path = cluster.ledger_path(&context.lighter_validator_key);
        let heavier_validator_ledger_path = cluster.ledger_path(&context.heaviest_validator_key);

        let (lighter_validator_latest_vote, _) = last_vote_in_tower(
            &lighter_validator_ledger_path,
            &context.lighter_validator_key,
        )
        .unwrap();

        info!(
            "Lighter validator's latest vote is for slot {}",
            lighter_validator_latest_vote
        );

        // Lighter partition should stop voting after detecting the heavier partition and try
        // to switch. Loop until we see a greater vote by the heavier validator than the last
        // vote made by the lighter validator on the lighter fork.
        let mut heavier_validator_latest_vote;
        let mut heavier_validator_latest_vote_hash;
        let heavier_blockstore = open_blockstore(&heavier_validator_ledger_path);
        loop {
            let (sanity_check_lighter_validator_latest_vote, _) = last_vote_in_tower(
                &lighter_validator_ledger_path,
                &context.lighter_validator_key,
            )
            .unwrap();

            // Lighter validator should stop voting, because `on_partition_resolved` is only
            // called after a propagation time where blocks from the other fork should have
            // finished propagating
            assert_eq!(
                sanity_check_lighter_validator_latest_vote,
                lighter_validator_latest_vote
            );

            let (new_heavier_validator_latest_vote, new_heavier_validator_latest_vote_hash) =
                last_vote_in_tower(
                    &heavier_validator_ledger_path,
                    &context.heaviest_validator_key,
                )
                .unwrap();

            heavier_validator_latest_vote = new_heavier_validator_latest_vote;
            heavier_validator_latest_vote_hash = new_heavier_validator_latest_vote_hash;

            // Latest vote for each validator should be on different forks
            assert_ne!(lighter_validator_latest_vote, heavier_validator_latest_vote);
            if heavier_validator_latest_vote > lighter_validator_latest_vote {
                let heavier_ancestors: HashSet<Slot> =
                    AncestorIterator::new(heavier_validator_latest_vote, &heavier_blockstore)
                        .collect();
                assert!(!heavier_ancestors.contains(&lighter_validator_latest_vote));
                break;
            }
        }

        info!("Checking to make sure lighter validator doesn't switch");
        let mut latest_slot = lighter_validator_latest_vote;

        // Number of chances the validator had to switch votes but didn't
        let mut total_voting_opportunities = 0;
        while total_voting_opportunities <= 5 {
            let (new_latest_slot, latest_slot_ancestors) =
                find_latest_replayed_slot_from_ledger(&lighter_validator_ledger_path, latest_slot);
            latest_slot = new_latest_slot;
            // Ensure `latest_slot` is on the other fork
            if latest_slot_ancestors.contains(&heavier_validator_latest_vote) {
                let tower = restore_tower(
                    &lighter_validator_ledger_path,
                    &context.lighter_validator_key,
                )
                .unwrap();
                // Check that there was an opportunity to vote
                if !tower.is_locked_out(latest_slot, &latest_slot_ancestors) {
                    // Ensure the lighter blockstore has not voted again
                    let new_lighter_validator_latest_vote = tower.last_voted_slot().unwrap();
                    assert_eq!(
                        new_lighter_validator_latest_vote,
                        lighter_validator_latest_vote
                    );
                    info!(
                        "Incrementing voting opportunities: {}",
                        total_voting_opportunities
                    );
                    total_voting_opportunities += 1;
                } else {
                    info!(
                        "Tower still locked out, can't vote for slot: {}",
                        latest_slot
                    );
                }
            } else if latest_slot > heavier_validator_latest_vote {
                warn!(
                    "validator is still generating blocks on its own fork, last processed slot: {}",
                    latest_slot
                );
            }
            sleep(Duration::from_millis(50));
        }

        // Make a vote from the killed validator for slot `heavier_validator_latest_vote` in gossip
        info!(
            "Simulate vote for slot: {} from dead validator",
            heavier_validator_latest_vote
        );
        let vote_keypair = &context
            .dead_validator_info
            .as_ref()
            .unwrap()
            .info
            .voting_keypair
            .clone();
        let node_keypair = &context
            .dead_validator_info
            .as_ref()
            .unwrap()
            .info
            .keypair
            .clone();
        let vote_ix = vote_instruction::vote(
            &vote_keypair.pubkey(),
            &vote_keypair.pubkey(),
            Vote::new(
                vec![heavier_validator_latest_vote],
                heavier_validator_latest_vote_hash,
            ),
        );

        let mut vote_tx = Transaction::new_with_payer(&[vote_ix], Some(&node_keypair.pubkey()));

        // Make the vote transaction with a random blockhash. Thus, the vote only lives in gossip but
        // never makes it into a block
        let blockhash = Hash::new_unique();
        vote_tx.partial_sign(&[node_keypair.as_ref()], blockhash);
        vote_tx.partial_sign(&[vote_keypair.as_ref()], blockhash);
        let heavier_node_gossip = cluster
            .get_contact_info(&context.heaviest_validator_key)
            .unwrap()
            .gossip;
        cluster_info::push_messages_to_peer(
            vec![CrdsValue::new_signed(
                CrdsData::Vote(
                    0,
                    crds_value::Vote::new(node_keypair.pubkey(), vote_tx, timestamp()),
                ),
                node_keypair,
            )],
            context
                .dead_validator_info
                .as_ref()
                .unwrap()
                .info
                .keypair
                .pubkey(),
            heavier_node_gossip,
            &SocketAddrSpace::Unspecified,
        )
        .unwrap();

        loop {
            // Wait for the lighter validator to switch to the heavier fork
            let (new_lighter_validator_latest_vote, _) = last_vote_in_tower(
                &lighter_validator_ledger_path,
                &context.lighter_validator_key,
            )
            .unwrap();

            if new_lighter_validator_latest_vote != lighter_validator_latest_vote {
                info!(
                    "Lighter validator switched forks at slot: {}",
                    new_lighter_validator_latest_vote
                );
                let (heavier_validator_latest_vote, _) = last_vote_in_tower(
                    &heavier_validator_ledger_path,
                    &context.heaviest_validator_key,
                )
                .unwrap();
                let (smaller, larger) =
                    if new_lighter_validator_latest_vote > heavier_validator_latest_vote {
                        (
                            heavier_validator_latest_vote,
                            new_lighter_validator_latest_vote,
                        )
                    } else {
                        (
                            new_lighter_validator_latest_vote,
                            heavier_validator_latest_vote,
                        )
                    };

                // Check the new vote is on the same fork as the heaviest fork
                let heavier_blockstore = open_blockstore(&heavier_validator_ledger_path);
                let larger_slot_ancestors: HashSet<Slot> =
                    AncestorIterator::new(larger, &heavier_blockstore)
                        .chain(std::iter::once(larger))
                        .collect();
                assert!(larger_slot_ancestors.contains(&smaller));
                break;
            } else {
                sleep(Duration::from_millis(50));
            }
        }
    };

    let ticks_per_slot = 8;
    run_kill_partition_switch_threshold(
        &[&[(failures_stake as usize, 0)]],
        partitions,
        // Partition long enough such that the first vote made by validator with
        // `alive_stake_3` won't be ingested due to BlockhashTooOld,
        None,
        Some(ticks_per_slot),
        PartitionContext::default(),
        on_partition_start,
        on_before_partition_resolved,
        on_partition_resolved,
    );
}

#[test]
#[serial]
fn test_listener_startup() {
    let mut config = ClusterConfig {
        node_stakes: vec![100; 1],
        cluster_lamports: 1_000,
        num_listeners: 3,
        validator_configs: make_identical_validator_configs(&ValidatorConfig::default(), 1),
        ..ClusterConfig::default()
    };
    let cluster = LocalCluster::new(&mut config, SocketAddrSpace::Unspecified);
    let cluster_nodes = discover_cluster(
        &cluster.entry_point_info.gossip,
        4,
        SocketAddrSpace::Unspecified,
    )
    .unwrap();
    assert_eq!(cluster_nodes.len(), 4);
}

fn find_latest_replayed_slot_from_ledger(
    ledger_path: &Path,
    mut latest_slot: Slot,
) -> (Slot, HashSet<Slot>) {
    loop {
        let mut blockstore = open_blockstore(ledger_path);
        // This is kind of a hack because we can't query for new frozen blocks over RPC
        // since the validator is not voting.
        let new_latest_slots: Vec<Slot> = blockstore
            .slot_meta_iterator(latest_slot)
            .unwrap()
            .filter_map(|(s, _)| if s > latest_slot { Some(s) } else { None })
            .collect();

        for new_latest_slot in new_latest_slots {
            latest_slot = new_latest_slot;
            info!("Checking latest_slot {}", latest_slot);
            // Wait for the slot to be fully received by the validator
            let entries;
            loop {
                info!("Waiting for slot {} to be full", latest_slot);
                if blockstore.is_full(latest_slot) {
                    entries = blockstore.get_slot_entries(latest_slot, 0).unwrap();
                    assert!(!entries.is_empty());
                    break;
                } else {
                    sleep(Duration::from_millis(50));
                    blockstore = open_blockstore(ledger_path);
                }
            }
            // Check the slot has been replayed
            let non_tick_entry = entries.into_iter().find(|e| !e.transactions.is_empty());
            if let Some(non_tick_entry) = non_tick_entry {
                // Wait for the slot to be replayed
                loop {
                    info!("Waiting for slot {} to be replayed", latest_slot);
                    if !blockstore
                        .map_transactions_to_statuses(
                            latest_slot,
                            non_tick_entry.transactions.clone().into_iter(),
                        )
                        .is_empty()
                    {
                        return (
                            latest_slot,
                            AncestorIterator::new(latest_slot, &blockstore).collect(),
                        );
                    } else {
                        sleep(Duration::from_millis(50));
                        blockstore = open_blockstore(ledger_path);
                    }
                }
            } else {
                info!(
                    "No transactions in slot {}, can't tell if it was replayed",
                    latest_slot
                );
            }
        }
        sleep(Duration::from_millis(50));
    }
}

use {
    crate::{
        stakes::{create_and_add_stakes, StakerInfo},
        unlocks::UnlockInfo,
    },
    solana_sdk::{genesis_config::GenesisConfig, native_token::LAMPORTS_PER_SOL},
};

// monthly from second year for 5 years
const UNLOCKS_MONTHLY_FOR_5_YEARS: UnlockInfo = UnlockInfo {
    cliff_fraction: 0.01666666667,
    cliff_years: 0.0,
    unlocks: 59,
    unlock_years: 4.9166666667,
    custodian: "8iQ7WaWL2Ez2PUd4rHcY1L8KPmz22YeMUUC45YBVrdab",
};
// monthly from second year for 2 years
const UNLOCKS_MONTHLY_FOR_2_YEARS: UnlockInfo = UnlockInfo {
    cliff_fraction: 0.04166666667,
    cliff_years: 0.0,
    unlocks: 23,
    unlock_years: 0.9583333333,
    custodian: "8iQ7WaWL2Ez2PUd4rHcY1L8KPmz22YeMUUC45YBVrdab",
};
// unlock immediately
const UNLOCKS_IMMEDIATELY: UnlockInfo = UnlockInfo {
    cliff_fraction: 1.0,
    cliff_years: 0.0,
    unlocks: 0,
    unlock_years: 0.0,
    custodian: "8iQ7WaWL2Ez2PUd4rHcY1L8KPmz22YeMUUC45YBVrdab",
};
// unlock immediately after 3 years
const UNLOCKS_AFTER_3_YEARS: UnlockInfo = UnlockInfo {
    cliff_fraction: 1.0,
    cliff_years: 3.0,
    unlocks: 0,
    unlock_years: 0.0,
    custodian: "8iQ7WaWL2Ez2PUd4rHcY1L8KPmz22YeMUUC45YBVrdab",
};
// Yearly for 10 years
const UNLOCKS_IN_10_YEARS: UnlockInfo = UnlockInfo {
    cliff_fraction: 0.1,
    cliff_years: 0.0,
    unlocks: 9,
    unlock_years: 9.0,
    custodian: "8iQ7WaWL2Ez2PUd4rHcY1L8KPmz22YeMUUC45YBVrdab",
};

// TODO: 40_000_000 is temporary, when mainnet is launched, edit this number
const VERIFIED_MINERS_AMOUNT: f64 = 24_000_000.0;
const UNVERIFIED_MINERS_AMOUNT: f64 = 16_000_000.0;
const MINERS_AMOUNT: f64 = VERIFIED_MINERS_AMOUNT + UNVERIFIED_MINERS_AMOUNT;
const REMITANO_RENEC_AMOUNT: f64 = MINERS_AMOUNT * 0.25;
const LIQUIDITY_AMOUNT: f64 = MINERS_AMOUNT * 0.05;
const BURST_MARKETING_AMOUNT: f64 = MINERS_AMOUNT * 0.05;
const LONGTERM_MARKETING_AMOUNT: f64 = MINERS_AMOUNT * 0.1;
const TREASURY_AMOUNT: f64 = MINERS_AMOUNT * 0.2;
const DAPP_SUPPORT_RENEC_AMOUNT: f64 = (VERIFIED_MINERS_AMOUNT + REMITANO_RENEC_AMOUNT + LIQUIDITY_AMOUNT + LIQUIDITY_AMOUNT + BURST_MARKETING_AMOUNT + LONGTERM_MARKETING_AMOUNT + TREASURY_AMOUNT) * 0.1;
const STAKERS: [&str; 8] = ["HRVwkvWMW51T7tVMsMAQUG6dVu8LW7fDH2DcModvr3TC", "BLeMPEXf5JqyWaCKjRffMwwrq5TqJTQWsrikdz8a6faZ", "3PqkJhLWuMgoXFQwa1AANd1UZ3Fpyz5PgmKucMXzkfQm", "FEsudMKqZE1Buz5PW3j1mjS96mHDnzUDqSh488nFLtV6", "BB7M6L1ZeodDud65WaP2GMRJMFMLh1xjwe9t3VfRWQWN", "7XSfx2HVDgSbfW9x5SNSjqWgqwyirmHqAcDdsvice7qN", "CYwsh6pqKSGgX1SZjgXn2SWrn9WtZfVnFuQs3v5UmNoB", "DKi2iEdsx9hiBrZY9xB4SVF62nz8rxTiJ8ANCq8wAA55"];
const WITHDRAWERS: [&str; 8] = ["8f4K7tjy5wHj24xWpZWbAWaX8G7mdvXHpG5deyxMq48i", "9rJMKsZJTLwSZBtirFmo89izHtZKo9pJFm7rjKvpczK8", "j7MnuJ7Ms7WmBUt4YznnE5X7sRbD14HKcda3iEknL5Z", "2ideThkLq8bN76wM6RdMaicjufg1QtzLyvu7eFGRYzE4", "AXDxHFFwx7dgyNwBjaA4QtBvmzKRjNw2359qKd9FvxfG", "AaBcsGAnsNxLz4B7CYWNFrwQMBynogtDanFbd7uSSP4q", "DcLxd25X2sxgGKSijDp6pqrHptZRsH3C7KKNvA4UapoJ", "7PHDKQS1Jus2kDhCVPNUnRKd4mpyEWp44gVdBVLHhBh2"];

pub const MINERS_AND_REMITANO_INFOS: &[StakerInfo] = &[
    StakerInfo {
        name: "miners",
        staker: "FKtEiKsTVjXiccHTxzHDyUkkzE9aTqoJxzWh5oY3kuno",
        lamports: VERIFIED_MINERS_AMOUNT as u64 * LAMPORTS_PER_SOL,
        withdrawer: Some("BDU8UZArkiUVzu9ReyXUaefqYNEZwJE7q2Ju95vWo6ZC"),
    },
    StakerInfo {
        name: "remitano",
        staker: "HsVaAaqQtweccoH8WGnGhSWpUgjk1Gmy5oRxbqq9eaow",
        lamports: REMITANO_RENEC_AMOUNT as u64 * LAMPORTS_PER_SOL,
        withdrawer: Some("4GD5xCfSJn97d1BCHVj47jsngdd3fJo7RWap8WxRKaBZ"),
    },
];
pub const LIQUIDITY_INFOS: &[StakerInfo] = &[
    StakerInfo {
        name: "liquidity",
        staker: "EufLViubSkHoFVrKjBrbpAkdJg7sGZYmYuCMEahWrGJ4",
        lamports: LIQUIDITY_AMOUNT as u64 * LAMPORTS_PER_SOL,
        withdrawer: Some("8t8WduDHEwcHFVyL9t5GQnWTEPc12L352xG6CAJuKgoU"),
    },
];
pub const REMITANO_BOUGHT_LIQUIDITY_INFOS: &[StakerInfo] = &[
    StakerInfo {
        name: "remitano bought liquidity",
        staker: "Bujir72W7XjkSuMB2N5MVeskHvfKAnBwETAseXJUV6Xs",
        lamports: LIQUIDITY_AMOUNT as u64 * LAMPORTS_PER_SOL,
        withdrawer: Some("3aJPDrQqDWDHXkzf9Zxic9GFrxbxdFAsCQAHVm4RXtDy"),
    },
];
pub const BURST_MARKETING_INFOS: &[StakerInfo] = &[
    StakerInfo {
        name: "burst marketing",
        staker: "FQYPWNSNTzqcMDKJ2Rqi8vgHWaRACvvUNkb3ogra9Rje",
        lamports: BURST_MARKETING_AMOUNT as u64 * LAMPORTS_PER_SOL,
        withdrawer: Some("6cdNkCWqazSAaHbUPGg4m7JKZouYdvMGRTjNWHmf8V5z"),
    },
];
pub const LONGTERM_MARKETING_INFOS: &[StakerInfo] = &[
    StakerInfo {
        name: "longterm marketing",
        staker: "25Cv2HQQdafjdkngAV1qBtuqkc2m9qXMmw37KGEtncQL",
        lamports: LONGTERM_MARKETING_AMOUNT as u64 * LAMPORTS_PER_SOL,
        withdrawer: Some("GgKdSn6ih1iE9VmyeLEicniAYwgnRj7tbJwRAF5qyvNT"),
    },
];
pub const TREASURY_INFOS: &[StakerInfo] = &[
    StakerInfo {
        name: "treasury",
        staker: "676ys3xDJgTLe5MtVAQJ8FnrWJA7kSsfkcFGQDAPZnSa",
        lamports: TREASURY_AMOUNT as u64 * LAMPORTS_PER_SOL,
        withdrawer: Some("39fT5Xbn2CXqPB2SqU9aMis1UA1fdwFZhqKgKntVjifz"),
    },
];
pub const DAPPS_STAKER_INFOS: &[StakerInfo] = &[
    StakerInfo {
        name: "dapps support",
        staker: "G1rkVu5JfWudnAaDCeDvr95eLQ4LdRWq4cSjyNjDVNwj",
        lamports: DAPP_SUPPORT_RENEC_AMOUNT as u64 * LAMPORTS_PER_SOL,
        withdrawer: Some("4PMtuSiZ8qzQT65NSmCwn1dXuHF55pjGyGtaMFWr8LfX"),
    },
];

fn add_stakes(
    genesis_config: &mut GenesisConfig,
    staker_infos: &[StakerInfo],
    unlock_info: &UnlockInfo,
) -> u64 {
    staker_infos
        .iter()
        .map(|staker_info| create_and_add_stakes(genesis_config, staker_info, unlock_info, None))
        .sum::<u64>()
}

pub fn add_genesis_accounts(genesis_config: &mut GenesisConfig, mut _issued_lamports: u64) {
    add_stakes(
        genesis_config,
        MINERS_AND_REMITANO_INFOS,
        &UNLOCKS_MONTHLY_FOR_5_YEARS,
    );
    add_stakes(
        genesis_config,
        LIQUIDITY_INFOS,
        &UNLOCKS_IMMEDIATELY,
    );
    add_stakes(
        genesis_config,
        REMITANO_BOUGHT_LIQUIDITY_INFOS,
        &UNLOCKS_IMMEDIATELY,
    );
    add_stakes(
        genesis_config,
        BURST_MARKETING_INFOS,
        &UNLOCKS_MONTHLY_FOR_2_YEARS,
    );
    add_stakes(
        genesis_config,
        LONGTERM_MARKETING_INFOS,
        &UNLOCKS_MONTHLY_FOR_5_YEARS,
    );
    add_stakes(
        genesis_config,
        TREASURY_INFOS,
        &UNLOCKS_AFTER_3_YEARS,
    );
    add_stakes(
        genesis_config,
        DAPPS_STAKER_INFOS,
        &UNLOCKS_IN_10_YEARS,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_genesis_accounts() {
        let mut genesis_config = GenesisConfig::default();

        assert_eq!(MINERS_AND_REMITANO_INFOS[0].lamports, 24_000_000 * LAMPORTS_PER_SOL);
        assert_eq!(MINERS_AND_REMITANO_INFOS[1].lamports, 10_000_000 * LAMPORTS_PER_SOL);
        assert_eq!(LIQUIDITY_INFOS[0].lamports, 2_000_000 * LAMPORTS_PER_SOL);
        assert_eq!(REMITANO_BOUGHT_LIQUIDITY_INFOS[0].lamports, 2_000_000 * LAMPORTS_PER_SOL);
        assert_eq!(BURST_MARKETING_INFOS[0].lamports, 2_000_000 * LAMPORTS_PER_SOL);
        assert_eq!(LONGTERM_MARKETING_INFOS[0].lamports, 4_000_000 * LAMPORTS_PER_SOL);
        assert_eq!(TREASURY_INFOS[0].lamports, 8_000_000 * LAMPORTS_PER_SOL);
        assert_eq!(DAPPS_STAKER_INFOS[0].lamports, 5_200_000 * LAMPORTS_PER_SOL);

        add_genesis_accounts(&mut genesis_config, 0);

        let lamports = genesis_config
            .accounts
            .iter()
            .map(|(_, account)| account.lamports)
            .sum::<u64>();

        assert_eq!(lamports, 57_200_000 * LAMPORTS_PER_SOL);
    }
}

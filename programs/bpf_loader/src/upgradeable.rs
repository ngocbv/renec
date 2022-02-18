solana_sdk::declare_builtin!(
    solana_sdk::bpf_loader_upgradeable::ID,
    solana_bpf_loader_upgradeable_program,
    renec_bpf_loader_program::process_instruction,
    upgradeable::id
);

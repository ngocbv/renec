renec_sdk::declare_builtin!(
    renec_sdk::bpf_loader_deprecated::ID,
    solana_bpf_loader_deprecated_program,
    renec_bpf_loader_program::process_instruction,
    deprecated::id
);

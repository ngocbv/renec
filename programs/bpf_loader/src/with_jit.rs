solana_sdk::declare_builtin!(
    solana_sdk::bpf_loader::ID,
    solana_bpf_loader_program_with_jit,
    renec_bpf_loader_program::process_instruction_jit
);

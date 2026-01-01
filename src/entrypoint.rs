use pinocchio::{
    account_info::AccountInfo, no_allocator, nostd_panic_handler, program_entrypoint,
    program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

use pinocchio_log::log;

use crate::processor;

program_entrypoint!(process_instruction);
no_allocator!();
nostd_panic_handler!();

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let discriminator = u32::from_le_bytes(
        instruction_data[0..4]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );

    match discriminator {
        0 => {
            log!("Instruction: CreateLookupTable");
            let untrusted_recent_slot = u64::from_le_bytes(
                instruction_data[4..12]
                    .try_into()
                    .map_err(|_| ProgramError::InvalidInstructionData)?,
            );

            let bump_seed = instruction_data[12];
            processor::process_create_lookup_table(
                program_id,
                accounts,
                untrusted_recent_slot,
                bump_seed,
            )?
        }
        1 => {
            log!("Instruction: FreezeLookupTable");
            processor::process_freeze_lookup_table(program_id, accounts)?
        }
        2 => {
            log!("Instruction: ExtendLookupTable");
            let address_len = u64::from_le_bytes(
                instruction_data[4..12]
                    .try_into()
                    .map_err(|_| ProgramError::InvalidInstructionData)?,
            ) as usize;

            let addresses_start = 12;
            let addresses_end = addresses_start + address_len * 32;

            if instruction_data.len() != addresses_end {
                return Err(ProgramError::InvalidInstructionData);
            }

            let raw_addresses = &instruction_data[addresses_start..addresses_end];

            processor::process_extend_lookup_table(program_id, accounts, raw_addresses)?
        }
        3 => {
            log!("Instruction: DeactivateLookupTable");
            processor::process_deactivate_lookup_table(program_id, accounts)?
        }
        4 => {
            log!("Instruction: CloseLookupTable");
            processor::process_close_lookup_table(program_id, accounts)?
        }
        _ => return Err(ProgramError::InvalidInstructionData),
    }

    Ok(())
}

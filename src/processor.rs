use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::{create_program_address, Pubkey, PUBKEY_BYTES},
    sysvars::{
        clock::{Clock, Slot},
        rent::Rent,
        slot_hashes::{SlotHashes, MAX_ENTRIES, SLOTHASHES_ID},
        Sysvar,
    },
    ProgramResult,
};
use pinocchio_log::log;
use pinocchio_system::instructions;

use crate::state::{
    serialize_new_lookup_table, LookupTableMeta, LOOKUP_TABLE_MAX_ADDRESSES, LOOKUP_TABLE_META_SIZE,
};

pub fn process_create_lookup_table(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    untrusted_recent_slot: Slot,
    bump_seed: u8,
) -> ProgramResult {
    let [lookup_table_info, authority_info, payer_info, slot_hashes_info, _system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !payer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if slot_hashes_info.key() != &SLOTHASHES_ID {
        return Err(ProgramError::InvalidArgument);
    }

    let derivation_slot = {
        let slot_hashes = SlotHashes::from_account_info(slot_hashes_info)?;
        if slot_hashes
            .entries()
            .iter()
            .any(|e| e.slot() == untrusted_recent_slot)
        {
            untrusted_recent_slot
        } else {
            log!("{} is not a recent slot", untrusted_recent_slot);
            return Err(ProgramError::InvalidInstructionData);
        }
    };

    let derived_table_seeds = &[
        authority_info.key().as_ref(),
        &derivation_slot.to_le_bytes(),
        &[bump_seed],
    ];

    let derived_table_key = create_program_address(derived_table_seeds, program_id)?;

    if lookup_table_info.key() != &derived_table_key {
        log!("Table address must match derived address");
        return Err(ProgramError::InvalidArgument);
    }

    if lookup_table_info.owner() == program_id {
        return Ok(());
    }

    let rent = <Rent as Sysvar>::get()?;
    let required_lamports = rent
        .minimum_balance(LOOKUP_TABLE_META_SIZE as usize)
        .max(1)
        .saturating_sub(lookup_table_info.lamports());

    let slot_bytes = derivation_slot.to_le_bytes();
    let bump_ref = [bump_seed];

    let seeds = [
        Seed::from(authority_info.key().as_ref()),
        Seed::from(&slot_bytes),
        Seed::from(&bump_ref),
    ];
    // Combined into one CPI, rather than the three CPI, will save cu
    instructions::CreateAccount {
        from: payer_info,
        to: lookup_table_info,
        lamports: required_lamports,
        space: LOOKUP_TABLE_META_SIZE as u64,
        owner: program_id,
    }
    .invoke_signed(&[Signer::from(&seeds)])?;

    let data = unsafe { lookup_table_info.borrow_mut_data_unchecked() };

    serialize_new_lookup_table(data, authority_info.key())?;

    Ok(())
}

pub fn process_freeze_lookup_table(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let [lookup_table_info, authority_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if lookup_table_info.owner() != program_id {
        log!("Lookup table owner should be the Address Lookup Table program");
        return Err(ProgramError::InvalidAccountOwner);
    }

    if !authority_info.is_signer() {
        log!("Authority account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let lookup_table_meta = {
        let data = unsafe { lookup_table_info.borrow_mut_data_unchecked() };
        let meta = unsafe { &mut *(data.as_mut_ptr().add(4) as *mut LookupTableMeta) };

        if meta.authority_tag == 0 {
            log!("Lookup table is already frozen");
            return Err(ProgramError::Immutable);
        }
        if meta.authority != *authority_info.key() {
            log!("Incorrect lookup table authority");
            return Err(ProgramError::IncorrectAuthority);
        }
        if meta.deactivation_slot != Slot::MAX {
            log!("Deactivated tables cannot be frozen");
            return Err(ProgramError::InvalidArgument);
        }
        if data.len() <= LOOKUP_TABLE_META_SIZE || data[LOOKUP_TABLE_META_SIZE..].is_empty() {
            log!("Empty lookup tables cannot be frozen");
            return Err(ProgramError::InvalidInstructionData);
        }

        meta
    };

    lookup_table_meta.authority_tag = 0;
    lookup_table_meta.authority = [0; 32];

    Ok(())
}

pub fn process_extend_lookup_table(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_addresses: &[u8],
) -> ProgramResult {
    let [lookup_table_info, authority_info, payer_info, _system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if lookup_table_info.owner() != program_id {
        log!("Lookup table owner should be the Address Lookup Table program");
        return Err(ProgramError::InvalidAccountOwner);
    }

    if !authority_info.is_signer() {
        log!("Authority account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (new_addresses_start_index, new_table_data_len) = {
        let data = unsafe { lookup_table_info.borrow_mut_data_unchecked() };
        let meta = unsafe { &mut *(data.as_mut_ptr().add(4) as *mut LookupTableMeta) };

        if meta.authority_tag == 0 {
            log!("Lookup table is already frozen");
            return Err(ProgramError::Immutable);
        }

        if &meta.authority != authority_info.key() {
            log!("Incorrect lookup table authority");
            return Err(ProgramError::IncorrectAuthority);
        }

        if meta.deactivation_slot != Slot::MAX {
            log!("Deactivated tables cannot be frozen");
            return Err(ProgramError::InvalidArgument);
        }

        let old_table_addresses_len = (data.len() - LOOKUP_TABLE_META_SIZE) / PUBKEY_BYTES;

        if old_table_addresses_len >= LOOKUP_TABLE_MAX_ADDRESSES {
            log!("Lookup table is full and cannot contain more addresses");
            return Err(ProgramError::InvalidArgument);
        }

        if new_addresses.is_empty() {
            log!("Must extend with at least one address");
            return Err(ProgramError::InvalidInstructionData);
        }

        let new_table_addresses_len =
            old_table_addresses_len.saturating_add(new_addresses.len() / PUBKEY_BYTES);

        if new_table_addresses_len > LOOKUP_TABLE_MAX_ADDRESSES {
            log!(
                "Extended lookup table length {} would exceed max capacity of {}",
                new_table_addresses_len,
                LOOKUP_TABLE_MAX_ADDRESSES,
            );
            return Err(ProgramError::InvalidInstructionData);
        }

        let clock = <Clock as Sysvar>::get()?;
        if clock.slot != meta.last_extended_slot {
            meta.last_extended_slot = clock.slot;
            meta.last_extended_slot_start_index = old_table_addresses_len as u8;
        }

        let new_table_data_len = LOOKUP_TABLE_META_SIZE
            .checked_add(new_table_addresses_len.saturating_mul(PUBKEY_BYTES))
            .ok_or(ProgramError::ArithmeticOverflow)?;

        (old_table_addresses_len, new_table_data_len)
    };

    if !lookup_table_info.is_writable() {
        return Err(ProgramError::Immutable);
    }

    lookup_table_info.resize(new_table_data_len)?;

    {
        let data = unsafe { lookup_table_info.borrow_mut_data_unchecked() };
        let offset = LOOKUP_TABLE_META_SIZE
            .checked_add(new_addresses_start_index.saturating_mul(PUBKEY_BYTES))
            .ok_or(ProgramError::ArithmeticOverflow)?;

        if offset >= data.len() {
            return Err(ProgramError::InvalidArgument);
        }
        data[offset..].copy_from_slice(new_addresses);
    }

    let rent = <Rent as Sysvar>::get()?;
    let required_lamports = rent
        .minimum_balance(new_table_data_len)
        .max(1)
        .saturating_sub(lookup_table_info.lamports());

    if required_lamports > 0 {
        if !payer_info.is_signer() {
            log!("Payer account must be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        instructions::Transfer {
            from: payer_info,
            to: lookup_table_info,
            lamports: required_lamports,
        }
        .invoke()?;
    }

    Ok(())
}

pub fn process_deactivate_lookup_table(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let [lookup_table_info, authority_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if lookup_table_info.owner() != program_id {
        log!("Lookup table owner should be the Address Lookup Table program");
        return Err(ProgramError::InvalidAccountOwner);
    }

    if !authority_info.is_signer() {
        log!("Authority account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let lookup_table_meta = {
        let data = unsafe { lookup_table_info.borrow_mut_data_unchecked() };
        let meta = unsafe { &mut *(data.as_mut_ptr().add(4) as *mut LookupTableMeta) };

        if meta.authority_tag == 0 {
            log!("Lookup table is already frozen");
            return Err(ProgramError::Immutable);
        }

        if &meta.authority != authority_info.key() {
            log!("Incorrect lookup table authority");
            return Err(ProgramError::IncorrectAuthority);
        }

        if meta.deactivation_slot != Slot::MAX {
            log!("Lookup table is already deactivated");
            return Err(ProgramError::InvalidArgument);
        }

        meta
    };

    let clock = <Clock as Sysvar>::get()?;
    lookup_table_meta.deactivation_slot = clock.slot;

    Ok(())
}

pub fn process_close_lookup_table(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let [lookup_table_info, authority_info, recipient_info, slot_hashes_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if lookup_table_info.owner() != program_id {
        log!("Lookup table owner should be the Address Lookup Table program");
        return Err(ProgramError::InvalidAccountOwner);
    }

    if !authority_info.is_signer() {
        log!("Authority account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if lookup_table_info.key() == recipient_info.key() {
        log!("Lookup table cannot be the recipient of reclaimed lamports");
        return Err(ProgramError::InvalidArgument);
    }

    {
        let data = unsafe { lookup_table_info.borrow_mut_data_unchecked() };
        let meta = unsafe { &mut *(data.as_mut_ptr().add(4) as *mut LookupTableMeta) };

        if meta.authority_tag == 0 {
            log!("Lookup table is frozen");
            return Err(ProgramError::Immutable);
        }
        if meta.authority != *authority_info.key() {
            log!("Incorrect lookup table authority");
            return Err(ProgramError::IncorrectAuthority);
        }

        let clock = <Clock as Sysvar>::get()?;
        let current_slot = clock.slot;

        // Want to avoid function call, they call a function in the reference

        if meta.deactivation_slot == Slot::MAX {
            log!("Lookup table is not deactivated");
            return Err(ProgramError::InvalidArgument);
        } else if meta.deactivation_slot == current_slot {
            log!(
                "Table cannot be closed until it's fully deactivated in {} blocks",
                MAX_ENTRIES.saturating_add(1)
            );
            return Err(ProgramError::InvalidArgument);
        } else {
            let slot_hashes = SlotHashes::from_account_info(slot_hashes_info)?;

            if let Some(slot_position) = slot_hashes.position(meta.deactivation_slot) {
                log!(
                    "Table cannot be closed until it's fully deactivated in {} blocks",
                    MAX_ENTRIES.saturating_sub(slot_position)
                );
                return Err(ProgramError::InvalidArgument);
            }
        }
    }

    let new_recipient_lamports = lookup_table_info
        .lamports()
        .checked_add(recipient_info.lamports())
        .ok_or::<ProgramError>(ProgramError::ArithmeticOverflow)?;

    if !recipient_info.is_writable() {
        return Err(ProgramError::Immutable);
    }

    *recipient_info.try_borrow_mut_lamports()? = new_recipient_lamports;

    if !lookup_table_info.is_writable() {
        return Err(ProgramError::Immutable);
    }

    lookup_table_info.resize(0)?;
    *lookup_table_info.try_borrow_mut_lamports()? = 0;

    Ok(())
}

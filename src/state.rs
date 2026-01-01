use pinocchio::program_error::ProgramError;
use pinocchio::pubkey::Pubkey;

pub const LOOKUP_TABLE_MAX_ADDRESSES: usize = 256;
pub const LOOKUP_TABLE_META_SIZE: usize = 56;

#[repr(C)]
pub struct LookupTableMeta {
    pub deactivation_slot: u64,
    pub last_extended_slot: u64,
    pub last_extended_slot_start_index: u8,
    pub authority_tag: u8,
    pub authority: Pubkey,
    pub _padding: u16,
}

#[inline]
pub fn serialize_new_lookup_table(
    data: &mut [u8],
    authority_key: &Pubkey,
) -> Result<(), ProgramError> {
    data[0..4].copy_from_slice(&1u32.to_le_bytes());

    let meta = unsafe { &mut *(data.as_mut_ptr().add(4) as *mut LookupTableMeta) };

    meta.deactivation_slot = u64::MAX;
    meta.last_extended_slot = 0;
    meta.last_extended_slot_start_index = 0;

    meta.authority_tag = 1;
    meta.authority = *authority_key;

    meta._padding = 0;

    Ok(())
}

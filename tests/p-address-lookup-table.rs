use mollusk_svm::{account_store::AccountStore, program, result::Check, sysvar, Mollusk};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use solana_program::example_mocks::solana_sdk::system_program;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

static ACCOUNTS: LazyLock<Mutex<InMemoryAccountStore>> =
    LazyLock::new(|| Mutex::new(InMemoryAccountStore::default()));

#[derive(Default)]
struct InMemoryAccountStore {
    accounts: HashMap<Pubkey, Account>,
}

impl AccountStore for InMemoryAccountStore {
    fn get_account(&self, pubkey: &Pubkey) -> Option<Account> {
        self.accounts.get(pubkey).cloned()
    }

    fn store_account(&mut self, pubkey: Pubkey, account: Account) {
        self.accounts.insert(pubkey, account);
    }
}

const PROGRAM_FILE_NAME: &str = "p_address_lookup_table";

const PROGRAM_ID: Pubkey = Pubkey::from_str_const("AddressLookupTab1e1111111111111111111111111");
const AUTHORITY: Pubkey = Pubkey::from_str_const("Authority1111111111111111111111111111111111");
const PAYER: Pubkey = Pubkey::from_str_const("Payer11111111111111111111111111111111111111");

#[test]
fn test_1_create_lookup_table() {
    let mut accounts = ACCOUNTS.lock().unwrap();

    let recent_slot: u64 = 0;
    let (lookup_table, bump) = Pubkey::find_program_address(
        &[AUTHORITY.as_ref(), &recent_slot.to_le_bytes()],
        &PROGRAM_ID,
    );
    let (slot_key, slot_account) =
        sysvar::Sysvars::default().keyed_account_for_slot_hashes_sysvar();

    accounts.store_account(AUTHORITY, Account::default());

    accounts.store_account(
        PAYER,
        Account {
            lamports: 1_000_000_000,
            ..Account::default()
        },
    );

    accounts.store_account(lookup_table, Account::default());
    accounts.store_account(slot_key, slot_account);
    accounts.store_account(
        program::keyed_account_for_system_program().0,
        program::keyed_account_for_system_program().1,
    );

    let create_descriminator: u32 = 0;
    let mut create_instruction_data = Vec::with_capacity(13);
    create_instruction_data.extend_from_slice(&create_descriminator.to_le_bytes());
    create_instruction_data.extend_from_slice(&recent_slot.to_le_bytes());
    create_instruction_data.extend_from_slice(&[bump]);

    let create_instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(lookup_table, false),
            AccountMeta::new_readonly(AUTHORITY, true),
            AccountMeta::new(PAYER, true),
            AccountMeta::new_readonly(slot_key, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: create_instruction_data,
    };

    let mollusk = Mollusk::new(&PROGRAM_ID, PROGRAM_FILE_NAME);
    let context = mollusk.with_context(accounts.accounts.clone());
    let result = context.process_and_validate_instruction(&create_instruction, &[Check::success()]);

    // preserve the state of the created_table_account
    accounts.store_account(
        lookup_table,
        result.get_account(&lookup_table).unwrap().clone(),
    );
}

#[test]
fn test_2_extend_lookup_program() {
    let mut accounts = ACCOUNTS.lock().unwrap();
    let recent_slot: u64 = 0;

    let (lookup_table, _bump) = Pubkey::find_program_address(
        &[AUTHORITY.as_ref(), &recent_slot.to_le_bytes()],
        &PROGRAM_ID,
    );

    let extend_descriminator: u32 = 2;
    let address_len: usize = 3;
    let new_addresses = [
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
    ];
    let mut extend_instruction_data = Vec::with_capacity(4 + 8 + new_addresses.len() * 32);
    extend_instruction_data.extend_from_slice(&extend_descriminator.to_le_bytes());
    extend_instruction_data.extend_from_slice(&address_len.to_le_bytes());
    extend_instruction_data.extend_from_slice(&new_addresses[0].as_ref());
    extend_instruction_data.extend_from_slice(&new_addresses[1].as_ref());
    extend_instruction_data.extend_from_slice(&new_addresses[2].as_ref());

    let extend_instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(lookup_table, false),
            AccountMeta::new_readonly(AUTHORITY, true),
            AccountMeta::new(PAYER, true),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: extend_instruction_data,
    };

    let mollusk = Mollusk::new(&PROGRAM_ID, PROGRAM_FILE_NAME);
    let context = mollusk.with_context(accounts.accounts.clone());
    let result = context.process_and_validate_instruction(&extend_instruction, &[Check::success()]);
    accounts.store_account(
        lookup_table,
        result.get_account(&lookup_table).unwrap().clone(),
    );
}

#[test]
fn test_3_freeze_lookup_table() {
    let accounts = ACCOUNTS.lock().unwrap();

    let recent_slot: u64 = 0;
    let (lookup_table, _bump) = Pubkey::find_program_address(
        &[AUTHORITY.as_ref(), &recent_slot.to_le_bytes()],
        &PROGRAM_ID,
    );

    let freeze_descriminator: u32 = 1;
    let mut freeze_instruction_data = Vec::with_capacity(4);
    freeze_instruction_data.extend_from_slice(&freeze_descriminator.to_le_bytes());

    let freeze_instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(lookup_table, false),
            AccountMeta::new_readonly(AUTHORITY, true),
        ],
        data: freeze_instruction_data,
    };

    let mollusk = Mollusk::new(&PROGRAM_ID, PROGRAM_FILE_NAME);
    let context = mollusk.with_context(accounts.accounts.clone());

    context.process_and_validate_instruction(&freeze_instruction, &[Check::success()]);

    //here we aren't passing the state of the table, its intentional, because we cant deactivate frozen account
}

#[test]
fn test_4_deactivate_lookup_table() {
    let mut accounts = ACCOUNTS.lock().unwrap();

    let recent_slot: u64 = 0;
    let (lookup_table, _bump) = Pubkey::find_program_address(
        &[AUTHORITY.as_ref(), &recent_slot.to_le_bytes()],
        &PROGRAM_ID,
    );

    let deactivate_descriminator: u32 = 3;
    let mut deactivate_instruction_data = Vec::with_capacity(4);
    deactivate_instruction_data.extend_from_slice(&deactivate_descriminator.to_le_bytes());

    let deactivate_instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(lookup_table, false),
            AccountMeta::new_readonly(AUTHORITY, true),
        ],
        data: deactivate_instruction_data,
    };

    let mollusk = Mollusk::new(&PROGRAM_ID, PROGRAM_FILE_NAME);
    let context = mollusk.with_context(accounts.accounts.clone());

    let result =
        context.process_and_validate_instruction(&deactivate_instruction, &[Check::success()]);
    accounts.store_account(
        lookup_table,
        result.get_account(&lookup_table).unwrap().clone(),
    );
}

#[test]
fn test_5_close_lookup_table() {
    let mut accounts = ACCOUNTS.lock().unwrap();

    let recent_slot: u64 = 0;
    let (lookup_table, _bump) = Pubkey::find_program_address(
        &[AUTHORITY.as_ref(), &recent_slot.to_le_bytes()],
        &PROGRAM_ID,
    );

    let (slot_key, _slot_account) =
        sysvar::Sysvars::default().keyed_account_for_slot_hashes_sysvar();

    let recipient = Pubkey::new_unique();
    accounts.store_account(recipient, Account::default());

    let mut tweaked_meta = accounts.get_account(&lookup_table).unwrap();
    tweaked_meta.data[4] = 42; //tweaking the deactivation slot so it wont be found in the recent slots
    accounts.store_account(lookup_table, tweaked_meta);

    let close_descriminator: u32 = 4;
    let mut close_instruction_data = Vec::with_capacity(4);
    close_instruction_data.extend_from_slice(&close_descriminator.to_le_bytes());

    let close_instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(lookup_table, false),
            AccountMeta::new_readonly(AUTHORITY, true),
            AccountMeta::new(recipient, false),
            AccountMeta::new_readonly(slot_key, false),
        ],
        data: close_instruction_data,
    };
    let mollusk = Mollusk::new(&PROGRAM_ID, PROGRAM_FILE_NAME);
    let context = mollusk.with_context(accounts.accounts.clone());

    context.process_and_validate_instruction(&close_instruction, &[Check::success()]);
}

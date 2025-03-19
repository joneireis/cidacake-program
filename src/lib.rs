#![allow(unexpected_cfgs)]

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    msg,
    program_error::ProgramError,
    program_pack::{Pack, Sealed, IsInitialized},
    program::{invoke},
};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CakeState {
    pub stock: u64,
    pub price: u64,
    pub owner: Pubkey,
}

impl Sealed for CakeState {}

impl IsInitialized for CakeState {
    fn is_initialized(&self) -> bool {
        self.owner != Pubkey::default() // Considera inicializado se o owner não for zero
    }
}

impl Pack for CakeState {
    const LEN: usize = 8 + 8 + 32; // stock (u64) + price (u64) + owner (Pubkey)

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let data = self.try_to_vec().expect("Failed to serialize CakeState");
        dst[..data.len()].copy_from_slice(&data);
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(src).map_err(|_| ProgramError::InvalidAccountData)
    }
}

entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey, // Prefixado com _ para evitar o aviso
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let owner = next_account_info(accounts_iter)?;
    let cake_account = next_account_info(accounts_iter)?;

    let owner_key = Pubkey::new_from_array([
        100, 109, 136, 234, 166, 99, 27, 86, 154, 136, 63, 60, 162, 177, 123, 132,
        56, 81, 240, 186, 95, 251, 93, 117, 71, 74, 145, 149, 7, 209, 144, 198
    ]);

    if owner.key != &owner_key {
        return Err(ProgramError::InvalidAccountData);
    }

    match instruction_data[0] {
        0 => {
            let mut cake_state = CakeState::unpack_unchecked(&cake_account.data.borrow())?;
            if cake_state.is_initialized() {
                return Err(ProgramError::AccountAlreadyInitialized);
            }
            cake_state.owner = *owner.key;
            cake_state.stock = 100;
            cake_state.price = 1_000_000;
            cake_state.pack_into_slice(&mut cake_account.data.borrow_mut());
        }
        1 => {
            let amount = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
            let mut cake_state = CakeState::unpack(&cake_account.data.borrow())?;
            cake_state.stock += amount;
            cake_state.pack_into_slice(&mut cake_account.data.borrow_mut());
            msg!("Estoque adicionado: {}", amount);
        }
        2 => {
            let new_price = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
            let mut cake_state = CakeState::unpack(&cake_account.data.borrow())?;
            cake_state.price = new_price;
            cake_state.pack_into_slice(&mut cake_account.data.borrow_mut());
            msg!("Preço atualizado: {}", new_price);
        }
        3 => {
            let buyer = next_account_info(accounts_iter)?;
            let buyer_token_account = next_account_info(accounts_iter)?;
            let owner_token_account = next_account_info(accounts_iter)?;
            let token_program = next_account_info(accounts_iter)?;

            let mut cake_state = CakeState::unpack(&cake_account.data.borrow())?;
            if cake_state.stock == 0 {
                return Err(ProgramError::InsufficientFunds);
            }

            let amount = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
            if amount > cake_state.stock {
                return Err(ProgramError::InsufficientFunds);
            }

            // Ajuste apenas para os decimais do token (9 decimais)
            let token_amount = amount * 1_000_000_000; // 10 tokens = 10 * 10^9 lamports
            cake_state.stock -= amount;
            cake_state.pack_into_slice(&mut cake_account.data.borrow_mut());

            // Log para depuração
            msg!("Transferindo {} tokens ({} lamports)", amount, token_amount);

            invoke(
                &spl_token::instruction::transfer(
                    token_program.key,
                    buyer_token_account.key,
                    owner_token_account.key,
                    buyer.key,
                    &[],
                    token_amount, // Use apenas a quantidade de tokens, ajustada
                )?,
                &[
                    buyer_token_account.clone(),
                    owner_token_account.clone(),
                    buyer.clone(),
                    token_program.clone(),
                ],
            )?;
            msg!("Venda realizada: {} bolos", amount);
        }
        _ => return Err(ProgramError::InvalidInstructionData),
    }

    Ok(())
}

// Testes apenas no ambiente host, não no target SBF
#[cfg(all(test, not(target_os = "solana")))]
mod tests {
    use super::*;
    use solana_program::{
        account_info::AccountInfo,
        pubkey::Pubkey,
        program_error::ProgramError,
        clock::Epoch,
    };

    #[test]
    fn test_add_stock() {
        let program_id = Pubkey::new_unique();
        let owner_key = Pubkey::new_from_array([
            100, 109, 136, 234, 166, 99, 27, 86, 154, 136, 63, 60, 162, 177, 123, 132,
            56, 81, 240, 186, 95, 251, 93, 117, 71, 74, 145, 149, 7, 209, 144, 198
        ]);
        let cake_key = Pubkey::new_unique();

        let mut owner_lamports = 0u64;
        let mut cake_lamports = 0u64;
        let mut owner_data = Vec::new();
        let mut cake_data = vec![0u8; CakeState::LEN];

        let owner_account = AccountInfo::new(
            &owner_key,
            false,
            false,
            &mut owner_lamports,
            &mut owner_data,
            &owner_key,
            false,
            Epoch::default(),
        );
        let cake_account = AccountInfo::new(
            &cake_key,
            false,
            true,
            &mut cake_lamports,
            &mut cake_data,
            &program_id,
            false,
            Epoch::default(),
        );

        let init_data = vec![0];
        process_instruction(&program_id, &[owner_account.clone(), cake_account.clone()], &init_data).unwrap();

        let mut instruction_data = vec![1];
        instruction_data.extend_from_slice(&50u64.to_le_bytes());
        let result = process_instruction(&program_id, &[owner_account, cake_account], &instruction_data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_owner() {
        let program_id = Pubkey::new_unique();
        let wrong_owner_key = Pubkey::new_unique();
        let cake_key = Pubkey::new_unique();

        let mut owner_lamports = 0u64;
        let mut cake_lamports = 0u64;
        let mut owner_data = Vec::new();
        let mut cake_data = Vec::new();

        let owner_account = AccountInfo::new(
            &wrong_owner_key,
            false,
            false,
            &mut owner_lamports,
            &mut owner_data,
            &wrong_owner_key,
            false,
            Epoch::default(),
        );
        let cake_account = AccountInfo::new(
            &cake_key,
            false,
            false,
            &mut cake_lamports,
            &mut cake_data,
            &wrong_owner_key,
            false,
            Epoch::default(),
        );

        let accounts = vec![owner_account, cake_account];
        let instruction_data = vec![0];
        let result = process_instruction(&program_id, &accounts, &instruction_data);
        assert_eq!(result, Err(ProgramError::InvalidAccountData));
    }
}
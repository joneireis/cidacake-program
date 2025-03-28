use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
    program_pack::{Pack, Sealed, IsInitialized},
    system_instruction,
    program::invoke_signed,
    sysvar::clock::Clock,
};
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum CakeError {
    #[error("Dados de instrução inválidos")]
    InvalidInstructionData,
    #[error("Programa incorreto")]
    IncorrectProgramId,
    #[error("Não autorizado")]
    Unauthorized,
    #[error("Estoque insuficiente")]
    InsufficientStock,
    #[error("Overflow aritmético")]
    ArithmeticOverflow,
}

impl From<CakeError> for ProgramError {
    fn from(error: CakeError) -> Self {
        match error {
            CakeError::InvalidInstructionData => ProgramError::InvalidInstructionData,
            CakeError::IncorrectProgramId => ProgramError::IncorrectProgramId,
            CakeError::Unauthorized => ProgramError::InvalidAccountData,
            CakeError::InsufficientStock => ProgramError::InsufficientFunds,
            CakeError::ArithmeticOverflow => ProgramError::ArithmeticOverflow,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CakeState {
    pub owner: Pubkey,
    pub product_counter: u64,
    pub history_counter: u64,
}

impl Sealed for CakeState {}

impl IsInitialized for CakeState {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Pack for CakeState {
    const LEN: usize = 48;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let slice = dst;
        slice[..32].copy_from_slice(self.owner.as_ref());
        slice[32..40].copy_from_slice(&self.product_counter.to_le_bytes());
        slice[40..48].copy_from_slice(&self.history_counter.to_le_bytes());
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        if src.len() != Self::LEN {
            return Err(CakeError::InvalidInstructionData.into());
        }
        let owner = Pubkey::try_from(&src[..32]).map_err(|_| CakeError::InvalidInstructionData)?;
        let product_counter = u64::from_le_bytes(src[32..40].try_into().unwrap());
        let history_counter = u64::from_le_bytes(src[40..48].try_into().unwrap());
        Ok(CakeState { owner, product_counter, history_counter })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Product {
    pub id: u64,
    pub name: [u8; 32],
    pub description: [u8; 128],
    pub price: u64,
    pub stock: u64,
}

impl Sealed for Product {}

impl IsInitialized for Product {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Pack for Product {
    const LEN: usize = 184;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let slice = dst;
        slice[..8].copy_from_slice(&self.id.to_le_bytes());
        slice[8..40].copy_from_slice(&self.name);
        slice[40..168].copy_from_slice(&self.description);
        slice[168..176].copy_from_slice(&self.price.to_le_bytes());
        slice[176..184].copy_from_slice(&self.stock.to_le_bytes());
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        if src.len() != Self::LEN {
            return Err(CakeError::InvalidInstructionData.into());
        }
        let id = u64::from_le_bytes(src[..8].try_into().unwrap());
        let mut name = [0u8; 32];
        name.copy_from_slice(&src[8..40]);
        let mut description = [0u8; 128];
        description.copy_from_slice(&src[40..168]);
        let price = u64::from_le_bytes(src[168..176].try_into().unwrap());
        let stock = u64::from_le_bytes(src[176..184].try_into().unwrap());
        Ok(Product { id, name, description, price, stock })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PurchaseHistory {
    pub product_id: u64,
    pub quantity: u64,
    pub total_price: u64,
    pub buyer: Pubkey,
    pub timestamp: i64,
}

impl Sealed for PurchaseHistory {}

impl IsInitialized for PurchaseHistory {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Pack for PurchaseHistory {
    const LEN: usize = 65;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let slice = dst;
        slice[0..8].copy_from_slice(&self.product_id.to_le_bytes());
        slice[8..16].copy_from_slice(&self.quantity.to_le_bytes());
        slice[16..24].copy_from_slice(&self.total_price.to_le_bytes());
        slice[24..56].copy_from_slice(self.buyer.as_ref());
        slice[56..64].copy_from_slice(&self.timestamp.to_le_bytes());
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        if src.len() != Self::LEN {
            return Err(CakeError::InvalidInstructionData.into());
        }
        let product_id = u64::from_le_bytes(src[0..8].try_into().unwrap());
        let quantity = u64::from_le_bytes(src[8..16].try_into().unwrap());
        let total_price = u64::from_le_bytes(src[16..24].try_into().unwrap());
        let buyer = Pubkey::try_from(&src[24..56]).map_err(|_| CakeError::InvalidInstructionData)?;
        let timestamp = i64::from_le_bytes(src[56..64].try_into().unwrap());
        Ok(PurchaseHistory { product_id, quantity, total_price, buyer, timestamp })
    }
}

fn get_pda(seeds: &[&[u8]], program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(seeds, program_id)
}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = instruction_data[0];
    let account_iter = &mut accounts.iter();

    match instruction {
        0 => {
            msg!("Instrução: initialize");
            let cake_account = next_account_info(account_iter)?;
            let owner = next_account_info(account_iter)?;
            let _payer = next_account_info(account_iter)?; // Prefixado com _ para evitar aviso
            let _system_program = next_account_info(account_iter)?; // Prefixado com _ para evitar aviso

            if cake_account.owner != program_id {
                return Err(CakeError::IncorrectProgramId.into());
            }

            if cake_account.data.borrow().len() != CakeState::LEN {
                return Err(CakeError::InvalidInstructionData.into());
            }

            let mut cake_state = CakeState::unpack_unchecked(&cake_account.data.borrow())?;
            cake_state.owner = *owner.key;
            cake_state.product_counter = 0;
            cake_state.history_counter = 0;
            CakeState::pack(cake_state, &mut cake_account.data.borrow_mut())?;
        }
        1 => {
            msg!("Instrução: add_product");
            if instruction_data.len() < 177 {
                return Err(CakeError::InvalidInstructionData.into());
            }
            let cake_account = next_account_info(account_iter)?;
            let product_account = next_account_info(account_iter)?;
            let owner = next_account_info(account_iter)?;
            let payer = next_account_info(account_iter)?;
            let system_program = next_account_info(account_iter)?;

            if cake_account.owner != program_id {
                return Err(CakeError::IncorrectProgramId.into());
            }

            let mut cake_state = CakeState::unpack(&cake_account.data.borrow())?;
            if cake_state.owner != *owner.key {
                return Err(CakeError::Unauthorized.into());
            }

            let product_id = cake_state.product_counter;
            let (expected_product_account, bump) = get_pda(&[b"product", &product_id.to_le_bytes()], program_id);

            if *product_account.key != expected_product_account {
                return Err(CakeError::InvalidInstructionData.into());
            }

            let rent = Rent::get()?;
            let rent_lamports = rent.minimum_balance(Product::LEN);

            let create_product_account_ix = system_instruction::create_account(
                payer.key,
                product_account.key,
                rent_lamports,
                Product::LEN as u64,
                program_id,
            );

            invoke_signed(
                &create_product_account_ix,
                &[payer.clone(), product_account.clone(), system_program.clone()],
                &[&[b"product", &product_id.to_le_bytes(), &[bump]]],
            )?;

            let name_str = String::from_utf8(instruction_data[1..33].to_vec())
                .map_err(|_| CakeError::InvalidInstructionData)?;
            let description_str = String::from_utf8(instruction_data[33..161].to_vec())
                .map_err(|_| CakeError::InvalidInstructionData)?;

            let mut name = [0u8; 32];
            let name_bytes = name_str.as_bytes();
            name[..name_bytes.len().min(32)].copy_from_slice(&name_bytes[..name_bytes.len().min(32)]);

            let mut description = [0u8; 128];
            let description_bytes = description_str.as_bytes();
            description[..description_bytes.len().min(128)].copy_from_slice(&description_bytes[..description_bytes.len().min(128)]);

            let price = u64::from_le_bytes(instruction_data[161..169].try_into().unwrap());
            let stock = u64::from_le_bytes(instruction_data[169..177].try_into().unwrap());

            let product = Product { id: product_id, name, description, price, stock };
            Product::pack(product, &mut product_account.data.borrow_mut())?;

            cake_state.product_counter += 1;
            CakeState::pack(cake_state, &mut cake_account.data.borrow_mut())?;
        }
        4 => {
            msg!("Instrução: sell, product_id={}, amount={}", u64::from_le_bytes(instruction_data[1..9].try_into().unwrap()), u64::from_le_bytes(instruction_data[9..17].try_into().unwrap()));
            if instruction_data.len() < 17 {
                return Err(CakeError::InvalidInstructionData.into());
            }
            let owner = next_account_info(account_iter)?;
            let cake_account = next_account_info(account_iter)?;
            let product_account = next_account_info(account_iter)?;
            let buyer = next_account_info(account_iter)?;
            let system_program = next_account_info(account_iter)?;
            let history_account = next_account_info(account_iter)?;
            let payer = next_account_info(account_iter)?;
            let clock = next_account_info(account_iter)?;
            let buyer_token = next_account_info(account_iter)?;
            let owner_token = next_account_info(account_iter)?;
            let token_program = next_account_info(account_iter)?;
            let usdt_mint = next_account_info(account_iter)?;

            if cake_account.owner != program_id {
                return Err(CakeError::IncorrectProgramId.into());
            }

            let mut cake_state = CakeState::unpack(&cake_account.data.borrow())?;
            if cake_state.owner != *owner.key {
                return Err(CakeError::Unauthorized.into());
            }

            let product_id = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
            let (expected_product_account, _) = get_pda(&[b"product", &product_id.to_le_bytes()], program_id);

            if *product_account.key != expected_product_account {
                return Err(CakeError::InvalidInstructionData.into());
            }

            let mut product = Product::unpack(&product_account.data.borrow())?;
            let amount = u64::from_le_bytes(instruction_data[9..17].try_into().unwrap());
            if amount > product.stock {
                return Err(CakeError::InsufficientStock.into());
            }

            let total_price = amount.checked_mul(product.price).ok_or(CakeError::ArithmeticOverflow)?;

            let buyer_token_data = spl_token::state::Account::unpack(&buyer_token.data.borrow())?;
            let owner_token_data = spl_token::state::Account::unpack(&owner_token.data.borrow())?;
            if buyer_token_data.mint != *usdt_mint.key || owner_token_data.mint != *usdt_mint.key {
                return Err(CakeError::InvalidInstructionData.into());
            }

            let transfer_ix = spl_token::instruction::transfer(
                token_program.key,
                buyer_token.key,
                owner_token.key,
                buyer.key,
                &[],
                total_price,
            )?;

            solana_program::program::invoke(
                &transfer_ix,
                &[buyer_token.clone(), owner_token.clone(), buyer.clone(), token_program.clone()],
            )?;

            product.stock -= amount;
            Product::pack(product, &mut product_account.data.borrow_mut())?;

            let rent = Rent::get()?;
            let rent_lamports = rent.minimum_balance(PurchaseHistory::LEN);

            let history_index = cake_state.history_counter;
            let (expected_history_account, bump) = get_pda(
                &[b"history", buyer.key.as_ref(), &product_id.to_le_bytes(), &history_index.to_le_bytes()],
                program_id,
            );

            if *history_account.key != expected_history_account {
                return Err(CakeError::InvalidInstructionData.into());
            }

            let create_history_account_ix = system_instruction::create_account(
                payer.key,
                history_account.key,
                rent_lamports,
                PurchaseHistory::LEN as u64,
                program_id,
            );

            invoke_signed(
                &create_history_account_ix,
                &[payer.clone(), history_account.clone(), system_program.clone()],
                &[&[b"history", buyer.key.as_ref(), &product_id.to_le_bytes(), &history_index.to_le_bytes(), &[bump]]],
            )?;

            let clock_info = Clock::from_account_info(clock)?;
            let timestamp = clock_info.unix_timestamp;

            let history_entry = PurchaseHistory {
                product_id,
                quantity: amount,
                total_price,
                buyer: *buyer.key,
                timestamp,
            };
            PurchaseHistory::pack(history_entry, &mut history_account.data.borrow_mut())?;

            cake_state.history_counter += 1;
            CakeState::pack(cake_state, &mut cake_account.data.borrow_mut())?;
        }
        _ => return Err(CakeError::InvalidInstructionData.into()),
    }
    Ok(())
}
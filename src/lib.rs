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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CakeState {
    pub owner: Pubkey,        // Proprietário da loja
    pub product_counter: u64, // Contador para gerar IDs únicos para produtos
    pub history_counter: u64, // Contador para gerar índices únicos para o histórico de compras
}

impl Sealed for CakeState {}

impl IsInitialized for CakeState {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Pack for CakeState {
    const LEN: usize = 48; // 32 (Pubkey) + 8 (u64) + 8 (u64)

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let slice = dst;
        slice[..32].copy_from_slice(self.owner.as_ref());
        slice[32..40].copy_from_slice(&self.product_counter.to_le_bytes());
        slice[40..48].copy_from_slice(&self.history_counter.to_le_bytes());
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        if src.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        let owner = Pubkey::try_from(&src[..32]).map_err(|_| ProgramError::InvalidAccountData)?;
        let product_counter = u64::from_le_bytes(src[32..40].try_into().unwrap());
        let history_counter = u64::from_le_bytes(src[40..48].try_into().unwrap());
        Ok(CakeState { owner, product_counter, history_counter })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Product {
    pub id: u64,              // ID único do produto
    pub name: [u8; 32],       // Nome do bolo (máximo 32 caracteres)
    pub description: [u8; 128], // Descrição (máximo 128 caracteres)
    pub price: u64,           // Preço em lamports
    pub stock: u64,           // Estoque disponível
}

impl Sealed for Product {}

impl IsInitialized for Product {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Pack for Product {
    const LEN: usize = 184; // 8 (id) + 32 (name) + 128 (description) + 8 (price) + 8 (stock)

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
            return Err(ProgramError::InvalidAccountData);
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
    pub product_id: u8,
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
    const LEN: usize = 57;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let slice = dst;
        slice[0] = self.product_id;
        slice[1..9].copy_from_slice(&self.quantity.to_le_bytes());
        slice[9..17].copy_from_slice(&self.total_price.to_le_bytes());
        slice[17..49].copy_from_slice(self.buyer.as_ref());
        slice[49..57].copy_from_slice(&self.timestamp.to_le_bytes());
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        if src.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        let product_id = src[0];
        let quantity = u64::from_le_bytes(src[1..9].try_into().unwrap());
        let total_price = u64::from_le_bytes(src[9..17].try_into().unwrap());
        let buyer = Pubkey::try_from(&src[17..49]).map_err(|_| ProgramError::InvalidAccountData)?;
        let timestamp = i64::from_le_bytes(src[49..57].try_into().unwrap());
        Ok(PurchaseHistory { product_id, quantity, total_price, buyer, timestamp })
    }
}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = instruction_data[0];
    match instruction {
        0 => {
            msg!("Instrução: initialize");
            let account_iter = &mut accounts.iter();
            let cake_account = next_account_info(account_iter)?;
            let owner = next_account_info(account_iter)?;
            let payer = next_account_info(account_iter)?;
            let system_program = next_account_info(account_iter)?;

            if cake_account.owner != program_id {
                return Err(ProgramError::IncorrectProgramId);
            }

            if cake_account.data.borrow().len() != CakeState::LEN {
                return Err(ProgramError::InvalidAccountData);
            }

            let mut cake_state = CakeState::unpack_unchecked(&cake_account.data.borrow())?;
            cake_state.owner = *owner.key;
            cake_state.product_counter = 0;
            cake_state.history_counter = 0; // Inicializa o history_counter
            CakeState::pack(cake_state, &mut cake_account.data.borrow_mut())?;
        }
        1 => {
            msg!("Instrução: add_product");
            let account_iter = &mut accounts.iter();
            let cake_account = next_account_info(account_iter)?;
            let product_account = next_account_info(account_iter)?;
            let owner = next_account_info(account_iter)?;
            let payer = next_account_info(account_iter)?;
            let system_program = next_account_info(account_iter)?;

            if cake_account.owner != program_id {
                return Err(ProgramError::IncorrectProgramId);
            }

            let mut cake_state = CakeState::unpack(&cake_account.data.borrow())?;
            if cake_state.owner != *owner.key {
                return Err(ProgramError::InvalidAccountData);
            }

            let product_id = cake_state.product_counter;
            let (expected_product_account, bump) = Pubkey::find_program_address(
                &[b"product", &product_id.to_le_bytes()],
                program_id,
            );

            if *product_account.key != expected_product_account {
                return Err(ProgramError::InvalidAccountData);
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
                &[
                    payer.clone(),
                    product_account.clone(),
                    system_program.clone(),
                ],
                &[&[b"product", &product_id.to_le_bytes(), &[bump]]],
            )?;

            let name_bytes = &instruction_data[1..33];
            let mut name = [0u8; 32];
            name.copy_from_slice(name_bytes);

            let description_bytes = &instruction_data[33..161];
            let mut description = [0u8; 128];
            description.copy_from_slice(description_bytes);

            let price = u64::from_le_bytes(instruction_data[161..169].try_into().unwrap());
            let initial_stock = u64::from_le_bytes(instruction_data[169..177].try_into().unwrap());

            let product = Product {
                id: product_id,
                name,
                description,
                price,
                stock: initial_stock,
            };
            Product::pack(product, &mut product_account.data.borrow_mut())?;

            cake_state.product_counter += 1;
            CakeState::pack(cake_state, &mut cake_account.data.borrow_mut())?;
        }
        2 => {
            msg!("Instrução: add_stock");
            let account_iter = &mut accounts.iter();
            let cake_account = next_account_info(account_iter)?;
            let product_account = next_account_info(account_iter)?;
            let owner = next_account_info(account_iter)?;

            if cake_account.owner != program_id {
                return Err(ProgramError::IncorrectProgramId);
            }

            let cake_state = CakeState::unpack(&cake_account.data.borrow())?;
            if cake_state.owner != *owner.key {
                return Err(ProgramError::InvalidAccountData);
            }

            let product_id = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
            let (expected_product_account, _) = Pubkey::find_program_address(
                &[b"product", &product_id.to_le_bytes()],
                program_id,
            );

            if *product_account.key != expected_product_account {
                return Err(ProgramError::InvalidAccountData);
            }

            let mut product = Product::unpack(&product_account.data.borrow())?;
            let amount = u64::from_le_bytes(instruction_data[9..17].try_into().unwrap());
            product.stock += amount;
            Product::pack(product, &mut product_account.data.borrow_mut())?;
        }
        3 => {
            msg!("Instrução: update_price");
            let account_iter = &mut accounts.iter();
            let cake_account = next_account_info(account_iter)?;
            let product_account = next_account_info(account_iter)?;
            let owner = next_account_info(account_iter)?;

            if cake_account.owner != program_id {
                return Err(ProgramError::IncorrectProgramId);
            }

            let cake_state = CakeState::unpack(&cake_account.data.borrow())?;
            if cake_state.owner != *owner.key {
                return Err(ProgramError::InvalidAccountData);
            }

            let product_id = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
            let (expected_product_account, _) = Pubkey::find_program_address(
                &[b"product", &product_id.to_le_bytes()],
                program_id,
            );

            if *product_account.key != expected_product_account {
                return Err(ProgramError::InvalidAccountData);
            }

            let mut product = Product::unpack(&product_account.data.borrow())?;
            let new_price = u64::from_le_bytes(instruction_data[9..17].try_into().unwrap());
            product.price = new_price;
            Product::pack(product, &mut product_account.data.borrow_mut())?;
        }
        4 => {
            msg!("Instrução: sell");
            let account_iter = &mut accounts.iter();
            let owner = next_account_info(account_iter)?; // 0: owner
            let cake_account = next_account_info(account_iter)?; // 1: cake_account
            let product_account = next_account_info(account_iter)?; // 2: product_account
            let buyer = next_account_info(account_iter)?; // 3: buyer
            let system_program = next_account_info(account_iter)?; // 4: system_program
            let history_account = next_account_info(account_iter)?; // 5: history_account
            let payer = next_account_info(account_iter)?; // 6: payer
            let clock = next_account_info(account_iter)?; // 7: clock

            if cake_account.owner != program_id {
                return Err(ProgramError::IncorrectProgramId);
            }

            let mut cake_state = CakeState::unpack(&cake_account.data.borrow())?;
            if cake_state.owner != *owner.key {
                return Err(ProgramError::InvalidAccountData);
            }

            let product_id = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
            let (expected_product_account, _) = Pubkey::find_program_address(
                &[b"product", &product_id.to_le_bytes()],
                program_id,
            );

            if *product_account.key != expected_product_account {
                return Err(ProgramError::InvalidAccountData);
            }

            let mut product = Product::unpack(&product_account.data.borrow())?;
            let amount = u64::from_le_bytes(instruction_data[9..17].try_into().unwrap());
            if amount > product.stock {
                return Err(ProgramError::InsufficientFunds);
            }

            let total_price = amount * product.price;

            // Transferir SOL do comprador para o owner
            let transfer_ix = system_instruction::transfer(
                buyer.key,
                owner.key,
                total_price,
            );
            solana_program::program::invoke(
                &transfer_ix,
                &[
                    buyer.clone(),
                    owner.clone(),
                    system_program.clone(),
                ],
            )?;

            product.stock -= amount;
            Product::pack(product, &mut product_account.data.borrow_mut())?;

            let rent = Rent::get()?;
            let rent_lamports = rent.minimum_balance(PurchaseHistory::LEN);

            let history_index = cake_state.history_counter; // Usar history_counter
            let (expected_history_account, bump) = Pubkey::find_program_address(
                &[
                    b"history",
                    buyer.key.as_ref(),
                    &product_id.to_le_bytes(),
                    &history_index.to_le_bytes(),
                ],
                program_id,
            );

            if *history_account.key != expected_history_account {
                return Err(ProgramError::InvalidAccountData);
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
                &[
                    payer.clone(),
                    history_account.clone(),
                    system_program.clone(),
                ],
                &[
                    &[
                        b"history",
                        buyer.key.as_ref(),
                        &product_id.to_le_bytes(),
                        &history_index.to_le_bytes(),
                        &[bump],
                    ],
                ],
            )?;

            let clock_info = Clock::from_account_info(clock)?;
            let timestamp = clock_info.unix_timestamp;

            let history_entry = PurchaseHistory {
                product_id: product_id as u8,
                quantity: amount,
                total_price,
                buyer: *buyer.key,
                timestamp,
            };
            PurchaseHistory::pack(history_entry, &mut history_account.data.borrow_mut())?;

            // Incrementar o history_counter após criar a conta de histórico
            cake_state.history_counter += 1;
            CakeState::pack(cake_state, &mut cake_account.data.borrow_mut())?;
        }
        5 => {
            msg!("Instrução: migrate_history");
            let account_iter = &mut accounts.iter();
            let history_account = next_account_info(account_iter)?;
            let payer = next_account_info(account_iter)?;
            let system_program = next_account_info(account_iter)?;
            let buyer = next_account_info(account_iter)?;

            let rent = Rent::get()?;
            let rent_lamports = rent.minimum_balance(PurchaseHistory::LEN);

            let create_history_account_ix = system_instruction::create_account(
                payer.key,
                history_account.key,
                rent_lamports,
                PurchaseHistory::LEN as u64,
                program_id,
            );

            invoke_signed(
                &create_history_account_ix,
                &[
                    payer.clone(),
                    history_account.clone(),
                    system_program.clone(),
                ],
                &[&[b"history", buyer.key.as_ref(), &[instruction_data[1]]]],
            )?;

            let product_id = instruction_data[2];
            let quantity = u64::from_le_bytes(instruction_data[3..11].try_into().unwrap());
            let total_price = u64::from_le_bytes(instruction_data[11..19].try_into().unwrap());
            let buyer_pubkey = Pubkey::try_from(&instruction_data[19..51]).map_err(|_| ProgramError::InvalidAccountData)?;
            let timestamp = i64::from_le_bytes(instruction_data[51..59].try_into().unwrap());

            let history_entry = PurchaseHistory {
                product_id,
                quantity,
                total_price,
                buyer: buyer_pubkey,
                timestamp,
            };
            PurchaseHistory::pack(history_entry, &mut history_account.data.borrow_mut())?;
        }
        6 => {
            msg!("Instrução: close_account");
            let account_iter = &mut accounts.iter();
            let cake_account = next_account_info(account_iter)?;
            let owner = next_account_info(account_iter)?;
            let system_program = next_account_info(account_iter)?;

            if cake_account.owner != program_id {
                return Err(ProgramError::IncorrectProgramId);
            }

            let cake_state = CakeState::unpack(&cake_account.data.borrow())?;
            if cake_state.owner != *owner.key {
                return Err(ProgramError::InvalidAccountData);
            }

            let lamports = cake_account.lamports();
            **cake_account.lamports.borrow_mut() = 0;
            **owner.lamports.borrow_mut() += lamports;

            let mut data = cake_account.data.borrow_mut();
            for byte in data.iter_mut() {
                *byte = 0;
            }
        }
        7 => {
            msg!("Instrução: close_product_account");
            let account_iter = &mut accounts.iter();
            let cake_account = next_account_info(account_iter)?; // 0: cake_account
            let product_account = next_account_info(account_iter)?; // 1: product_account
            let owner = next_account_info(account_iter)?; // 2: owner
            let system_program = next_account_info(account_iter)?; // 3: system_program

            if cake_account.owner != program_id {
                return Err(ProgramError::IncorrectProgramId);
            }

            let cake_state = CakeState::unpack(&cake_account.data.borrow())?;
            if cake_state.owner != *owner.key {
                return Err(ProgramError::InvalidAccountData);
            }

            let product_id = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
            let (expected_product_account, _) = Pubkey::find_program_address(
                &[b"product", &product_id.to_le_bytes()],
                program_id,
            );

            if *product_account.key != expected_product_account {
                return Err(ProgramError::InvalidAccountData);
            }

            if product_account.owner != program_id {
                return Err(ProgramError::IncorrectProgramId);
            }

            let lamports = product_account.lamports();
            **product_account.lamports.borrow_mut() = 0;
            **owner.lamports.borrow_mut() += lamports;

            let mut data = product_account.data.borrow_mut();
            for byte in data.iter_mut() {
                *byte = 0;
            }
        }
        _ => return Err(ProgramError::InvalidInstructionData),
    }
    Ok(())
}
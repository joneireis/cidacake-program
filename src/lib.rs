use solana_program::{
    account_info::{next_account_info, AccountInfo, Account},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
    program_pack::{Pack, Sealed, IsInitialized},
    system_instruction,
    system_program,
    program::invoke_signed,
    sysvar::clock::Clock,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CakeState {
    pub stock: u64,
    pub price: u64,
    pub owner: Pubkey,
    pub history_counter: u64, // Novo campo para contar o número de compras
}

impl Sealed for CakeState {}

impl IsInitialized for CakeState {
    fn is_initialized(&self) -> bool {
        true // Consideramos que a conta está inicializada se os dados têm o tamanho correto
    }
}

impl Pack for CakeState {
    const LEN: usize = 56; // Aumentado para incluir history_counter (48 + 8 bytes)

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let slice = dst;
        slice[..8].copy_from_slice(&self.stock.to_le_bytes());
        slice[8..16].copy_from_slice(&self.price.to_le_bytes());
        slice[16..48].copy_from_slice(self.owner.as_ref());
        slice[48..56].copy_from_slice(&self.history_counter.to_le_bytes());
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        if src.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        let stock = u64::from_le_bytes(src[..8].try_into().unwrap());
        let price = u64::from_le_bytes(src[8..16].try_into().unwrap());
        let owner = Pubkey::try_from(&src[16..48]).map_err(|_| ProgramError::InvalidAccountData)?;
        let history_counter = u64::from_le_bytes(src[48..56].try_into().unwrap());
        Ok(CakeState { stock, price, owner, history_counter })
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

            // Verificar se a conta tem o tamanho correto
            if cake_account.data.borrow().len() != CakeState::LEN {
                return Err(ProgramError::InvalidAccountData);
            }

            // Inicializar os dados da conta
            let mut cake_state = CakeState::unpack_unchecked(&cake_account.data.borrow())?;
            cake_state.stock = 100;
            cake_state.price = 1_000_000;
            cake_state.owner = *owner.key;
            cake_state.history_counter = 0; // Inicializa o contador
            CakeState::pack(cake_state, &mut cake_account.data.borrow_mut())?;
        }
        1 => {
            msg!("Instrução: add_stock");
            let account_iter = &mut accounts.iter();
            let cake_account = next_account_info(account_iter)?;
            let owner = next_account_info(account_iter)?;

            if cake_account.owner != program_id {
                return Err(ProgramError::IncorrectProgramId);
            }

            let mut cake_state = CakeState::unpack(&cake_account.data.borrow())?;
            if cake_state.owner != *owner.key {
                return Err(ProgramError::InvalidAccountData);
            }

            let amount = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
            cake_state.stock += amount;
            CakeState::pack(cake_state, &mut cake_account.data.borrow_mut())?;
        }
        2 => {
            msg!("Instrução: update_price");
            let account_iter = &mut accounts.iter();
            let cake_account = next_account_info(account_iter)?;
            let owner = next_account_info(account_iter)?;

            if cake_account.owner != program_id {
                return Err(ProgramError::IncorrectProgramId);
            }

            let mut cake_state = CakeState::unpack(&cake_account.data.borrow())?;
            if cake_state.owner != *owner.key {
                return Err(ProgramError::InvalidAccountData);
            }

            let new_price = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
            cake_state.price = new_price;
            CakeState::pack(cake_state, &mut cake_account.data.borrow_mut())?;
        }
        3 => {
            msg!("Instrução: sell");
            let account_iter = &mut accounts.iter();
            let owner = next_account_info(account_iter)?; // 0: owner
            let cake_account = next_account_info(account_iter)?; // 1: cake_account
            let buyer = next_account_info(account_iter)?; // 2: buyer
            let system_program = next_account_info(account_iter)?; // 3: system_program
            let history_account = next_account_info(account_iter)?; // 4: history_account
            let payer = next_account_info(account_iter)?; // 5: payer
            let clock = next_account_info(account_iter)?; // 6: clock

            if cake_account.owner != program_id {
                return Err(ProgramError::IncorrectProgramId);
            }

            let mut cake_state = CakeState::unpack(&cake_account.data.borrow())?;
            if cake_state.owner != *owner.key {
                return Err(ProgramError::InvalidAccountData);
            }

            let amount = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
            if amount > cake_state.stock {
                return Err(ProgramError::InsufficientFunds);
            }

            let total_price = amount * cake_state.price;

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

            // Atualizar o estoque
            cake_state.stock -= amount;

            // Incrementar o contador de histórico
            let history_index = cake_state.history_counter;
            cake_state.history_counter += 1;
            CakeState::pack(cake_state, &mut cake_account.data.borrow_mut())?;

            // Criar uma nova conta de histórico
            let rent = Rent::get()?;
            let rent_lamports = rent.minimum_balance(PurchaseHistory::LEN);

            let create_history_account_ix = system_instruction::create_account(
                payer.key,
                history_account.key,
                rent_lamports,
                PurchaseHistory::LEN as u64,
                program_id,
            );

            // Obter o timestamp atual para usar como parte das sementes
            let clock_info = Clock::from_account_info(clock)?;
            let timestamp = clock_info.unix_timestamp;

            // Derivar o bump para o endereço da history_account, usando o history_index
            let (expected_history_account, bump) = Pubkey::find_program_address(
                &[
                    b"history",
                    buyer.key.as_ref(),
                    &[instruction_data[9]], // product_id
                    &history_index.to_le_bytes(), // history_index como bytes
                ],
                program_id,
            );

            // Verificar se o history_account fornecido corresponde ao esperado
            if *history_account.key != expected_history_account {
                return Err(ProgramError::InvalidAccountData);
            }

            // Incluir o bump nas sementes
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
                        &[instruction_data[9]],
                        &history_index.to_le_bytes(),
                        &[bump],
                    ],
                ],
            )?;

            let product_id = instruction_data[9]; // O product_id é enviado como o próximo byte após a quantidade

            let history_entry = PurchaseHistory {
                product_id,
                quantity: amount,
                total_price,
                buyer: *buyer.key,
                timestamp,
            };
            PurchaseHistory::pack(history_entry, &mut history_account.data.borrow_mut())?;
        }
        4 => {
            msg!("Instrução: migrate_history");
            let account_iter = &mut accounts.iter();
            let history_account = next_account_info(account_iter)?;
            let payer = next_account_info(account_iter)?;
            let system_program = next_account_info(account_iter)?;
            let buyer = next_account_info(account_iter)?;

            // Criar uma nova conta de histórico
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

            let product_id = instruction_data[2]; // u8 (1 byte)
            let quantity = u64::from_le_bytes(instruction_data[3..11].try_into().unwrap()); // u64 (8 bytes)
            let total_price = u64::from_le_bytes(instruction_data[11..19].try_into().unwrap()); // u64 (8 bytes)
            let buyer_pubkey = Pubkey::try_from(&instruction_data[19..51]).map_err(|_| ProgramError::InvalidAccountData)?; // Pubkey (32 bytes)
            let timestamp = i64::from_le_bytes(instruction_data[51..59].try_into().unwrap()); // i64 (8 bytes)

            let history_entry = PurchaseHistory {
                product_id,
                quantity,
                total_price,
                buyer: buyer_pubkey,
                timestamp,
            };
            PurchaseHistory::pack(history_entry, &mut history_account.data.borrow_mut())?;
        }
        5 => {
            msg!("Instrução: close_account");
            let account_iter = &mut accounts.iter();
            let cake_account = next_account_info(account_iter)?; // 0: cake_account
            let owner = next_account_info(account_iter)?; // 1: owner
            let system_program = next_account_info(account_iter)?; // 2: system_program

            msg!("Verificando se a conta pertence ao programa...");
            if cake_account.owner != program_id {
                msg!("Erro: Conta não pertence ao programa. Owner: {}, Program ID: {}", cake_account.owner, program_id);
                return Err(ProgramError::IncorrectProgramId);
            }

            msg!("Deserializando o estado da conta...");
            let cake_state = CakeState::unpack(&cake_account.data.borrow())?;
            msg!("Estado deserializado: stock: {}, price: {}, owner: {}", cake_state.stock, cake_state.price, cake_state.owner);

            msg!("Verificando se o owner é o proprietário correto...");
            if cake_state.owner != *owner.key {
                msg!("Erro: Owner não corresponde. Estado owner: {}, Owner fornecido: {}", cake_state.owner, owner.key);
                return Err(ProgramError::InvalidAccountData);
            }

            msg!("Transferindo lamports para o owner...");
            let lamports = cake_account.lamports();
            msg!("Lamports na conta: {}", lamports);
            {
                msg!("Obtendo borrow mutável para cake_account.lamports...");
                let mut cake_lamports = cake_account.lamports.borrow_mut();
                msg!("Obtendo borrow mutável para owner.lamports...");
                let mut owner_lamports = owner.lamports.borrow_mut();
                msg!("Zerando lamports da conta e transferindo para o owner...");
                **cake_lamports = 0;
                **owner_lamports += lamports;
                msg!("Transferência de lamports concluída.");
            }

            msg!("Obtendo borrow mutável para os dados da conta...");
            let mut data = cake_account.data.borrow_mut();
            msg!("Zerando os dados da conta...");
            for byte in data.iter_mut() {
                *byte = 0;
            }
            msg!("Dados zerados. Conta fechada com sucesso.");
        }
        _ => return Err(ProgramError::InvalidInstructionData),
    }
    Ok(())
}
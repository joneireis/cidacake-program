#[allow(unexpected_cfgs)]
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::{invoke},
    program_error::ProgramError,
    program_pack::{Pack, Sealed, IsInitialized},
    pubkey::Pubkey,
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
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let owner = next_account_info(accounts_iter)?;
    let cake_account = next_account_info(accounts_iter)?;

    let owner_key = "yG9KfVSMZaMZHSY48KKxpvtdPZhbAMUsYsAfKZDUkW5"
        .parse::<Pubkey>()
        .map_err(|_| ProgramError::InvalidArgument)?;

    if owner.key != &owner_key {
        return Err(ProgramError::InvalidAccountData);
    }

    match instruction_data[0] {
        0 => {
            // Verificar se a conta já está inicializada
            {
                let cake_state_data = &cake_account.data.borrow();
                if cake_state_data.len() >= CakeState::LEN {
                    let cake_state = CakeState::unpack_from_slice(cake_state_data)?;
                    if cake_state.is_initialized() {
                        return Err(ProgramError::AccountAlreadyInitialized);
                    }
                }
            }

            // Inicializar a conta
            let cake_state = CakeState {
                stock: 100,
                price: 1_000_000,
                owner: *owner.key,
            };
            cake_state.pack_into_slice(&mut cake_account.data.borrow_mut());
        }
        1 => {
            let mut cake_state = CakeState::unpack(&cake_account.data.borrow())?;
            let amount = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
            cake_state.stock += amount;
            cake_state.pack_into_slice(&mut cake_account.data.borrow_mut());
            msg!("Estoque adicionado: {}", amount);
        }
        2 => {
            let mut cake_state = CakeState::unpack(&cake_account.data.borrow())?;
            let new_price = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
            cake_state.price = new_price;
            cake_state.pack_into_slice(&mut cake_account.data.borrow_mut());
            msg!("Preço atualizado: {}", new_price);
        }
        3 => {
            let buyer = next_account_info(accounts_iter)?;
            let buyer_token_account = next_account_info(accounts_iter)?;
            let owner_token_account = next_account_info(accounts_iter)?;
            let token_program = next_account_info(accounts_iter)?;
            let system_program = next_account_info(accounts_iter)?;

            let mut cake_state = CakeState::unpack(&cake_account.data.borrow())?;
            if cake_state.stock == 0 {
                return Err(ProgramError::InsufficientFunds);
            }

            let amount = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
            if amount > cake_state.stock {
                return Err(ProgramError::InsufficientFunds);
            }

            let token_amount = amount * 1_000_000_000;
            let total_cost = cake_state.price * amount;

            if **buyer.lamports.borrow() < total_cost {
                msg!("Erro: Buyer não tem SOL suficiente. Necessário: {}, Disponível: {}", total_cost, **buyer.lamports.borrow());
                return Err(ProgramError::InsufficientFunds);
            }

            // Transferir SOL do buyer para o owner usando SystemProgram::transfer
            invoke(
                &solana_program::system_instruction::transfer(
                    buyer.key,
                    owner.key,
                    total_cost,
                ),
                &[
                    buyer.clone(),
                    owner.clone(),
                    system_program.clone(),
                ],
            )?;

            cake_state.stock -= amount;
            cake_state.pack_into_slice(&mut cake_account.data.borrow_mut());

            msg!("Transferindo {} tokens ({} lamports)", amount, token_amount);
            msg!("Transferindo {} lamports (SOL) do buyer para o owner", total_cost);

            invoke(
                &spl_token::instruction::transfer(
                    token_program.key,
                    buyer_token_account.key,
                    owner_token_account.key,
                    buyer.key,
                    &[],
                    token_amount,
                )?,
                &[
                    buyer_token_account.clone(),
                    owner_token_account.clone(),
                    buyer.clone(),
                    token_program.clone(),
                ],
            )?;
            msg!("Venda realizada: {} bolos por {} lamports", amount, total_cost);
        }
        _ => return Err(ProgramError::InvalidInstructionData),
    }

    Ok(())
}
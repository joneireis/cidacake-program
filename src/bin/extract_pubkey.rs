use std::fs::File;
use std::io::{Read, Write};
use solana_sdk::signature::{Keypair, Signer};
use tempfile::NamedTempFile;

// Usar cfg para compilar apenas no ambiente host (não BPF)
#[cfg(not(target_arch = "bpf"))]
use solana_sdk::signature::keypair_from_seed;

fn main() {
    let wallet_path = "/Users/joneirocha/cidacake-wallet.json";
    let mut file = match File::open(wallet_path) {
        Ok(file) => file,
        Err(e) => panic!("Erro ao abrir o arquivo {}: {}", wallet_path, e),
    };

    let mut data = String::new();
    file.read_to_string(&mut data).expect("Não foi possível ler o arquivo");

    let secret_key: Vec<u8> = serde_json::from_str(&data)
        .expect("Formato JSON inválido ou não é um array de bytes");

    if secret_key.len() != 64 {
        panic!(
            "O array deve conter 64 bytes (chave privada Solana), encontrado: {}",
            secret_key.len()
        );
    }

    let keypair = Keypair::from_bytes(&secret_key)
        .expect("Não foi possível criar Keypair a partir dos bytes");

    let public_key = keypair.pubkey();
    let public_key_bytes = public_key.to_bytes();

    println!("pub const OWNER_KEY: [u8; 32] = {:?};", public_key_bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_pubkey() {
        // Usar uma chave fixa para o teste apenas no ambiente host
        #[cfg(not(target_arch = "bpf"))]
        {
            let seed = [1u8; 32]; // Seed fixo para teste
            let keypair = keypair_from_seed(&seed).expect("Não foi possível criar Keypair a partir do seed");
            let secret_key = keypair.to_bytes();

            let mut temp_file =
                NamedTempFile::new().expect("Não foi possível criar arquivo temporário");
            let json_data = serde_json::to_string(&secret_key.to_vec()).unwrap();
            temp_file
                .write_all(json_data.as_bytes())
                .expect("Não foi possível escrever no arquivo");

            let mut file = File::open(temp_file.path()).expect("Não foi possível abrir o arquivo");
            let mut data = String::new();
            file.read_to_string(&mut data).expect("Não foi possível ler o arquivo");

            let parsed_key: Vec<u8> = serde_json::from_str(&data).expect("Formato JSON inválido");
            assert_eq!(parsed_key.len(), 64);

            let extracted_keypair = Keypair::from_bytes(&parsed_key).expect("Keypair inválido");
            let public_key_bytes = extracted_keypair.pubkey().to_bytes();
            assert_eq!(public_key_bytes.len(), 32);
            assert_eq!(public_key_bytes, keypair.pubkey().to_bytes());
        }

        #[cfg(target_arch = "bpf")]
        {
            // Placeholder para BPF (não deve ser executado aqui)
            assert!(true);
        }
    }
}
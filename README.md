# CidaCake Program

Este é um contrato inteligente na rede Solana para a empresa fictícia "CidaCake", projetado para gerenciar o estoque e vendas de bolos. O programa permite que o proprietário inicialize o estoque, adicione mais bolos, atualize o preço e realize vendas com pagamentos em tokens USDT ou USDC.

## Descrição

O programa é escrito em Rust e usa a biblioteca `solana-program` para operar na blockchain Solana. Ele gerencia uma estrutura de dados chamada `CakeState`, que armazena:
- `stock`: Quantidade de bolos em estoque (u64).
- `price`: Preço por bolo em lamports de USDT/USDC (u64).
- `owner`: Chave pública do proprietário autorizado (Pubkey).

### Funcionalidades
- **Inicialização**: Define o estoque inicial (100 bolos) e preço (1 milhão de lamports).
- **Adicionar Estoque**: Permite ao proprietário incrementar o estoque.
- **Atualizar Preço**: Permite ao proprietário mudar o preço dos bolos.
- **Vender Bolos**: Decrementa o estoque e transfere tokens USDT/USDC do comprador para o proprietário.

### Dependências
- `solana-program`: Biblioteca principal para programas Solana.
- `borsh`: Serialização/desserialização de dados.
- `spl-token`: Integração com tokens SPL (USDT/USDC).

## Pré-requisitos

- **Rust**: Versão 1.75 ou superior.
- **Solana CLI**: Versão 1.18.x (`solana --version` para verificar).
  - Instale com: `sh -c "$(curl -sSfL https://release.solana.com/stable/install)"`.
- **Carteira com Saldo**: Uma carteira com SOL na Devnet para pagar taxas de implantação.
- **Git**: Para versionamento.

## Estrutura do Projeto

- `src/lib.rs`: Código principal do contrato Solana.
- `src/bin/extract_pubkey.rs`: Ferramenta auxiliar para extrair a chave pública de um arquivo JSON.
- `Cargo.toml`: Configuração do projeto e dependências.

## Instalação

1. **Clone o Repositório**:
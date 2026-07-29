// src/integrations/solana/client.rs

// use solana_client::rpc_client::RpcClient;
// use solana_sdk::{
//     pubkey::Pubkey,
//     signature::{Keypair, Signer},
//     transaction::Transaction,
//     instruction::{Instruction, AccountMeta},
// };
// use borsh::{BorshSerialize, BorshDeserialize};

pub struct SolanaAgentClient {
    // rpc: RpcClient,
    // payer: Keypair,
    // program_id: Pubkey,
}

impl SolanaAgentClient {
    pub fn new(_rpc_url: &str, _payer_privkey: &str, _program_id: String) -> Self {
        // let bytes = hex::decode(payer_privkey).unwrap();
        // let payer = Keypair::from_bytes(&bytes).unwrap();
        Self {
            // rpc: RpcClient::new(rpc_url.to_string()),
            // payer,
            // program_id,
        }
    }

    /// Inicializa um agente na Solana
    pub async fn initialize_agent(&self, _agent_id: [u8; 32], _arweave_txid: [u8; 32]) -> Result<String, String> {
        // // Calcular PDA do agente
        // let (pda, _bump) = Pubkey::find_program_address(&[b"agent", &agent_id], &self.program_id);

        // // Criar instrução
        // let instruction = Instruction {
        //     program_id: self.program_id,
        //     accounts: vec![
        //         AccountMeta::new(pda, false),
        //         AccountMeta::new(self.payer.pubkey(), true),
        //         AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        //     ],
        //     data: vec![0; 0], // STUB: serializar Initialize
        // };

        // let tx = Transaction::new_signed_with_payer(
        //     &[instruction],
        //     Some(&self.payer.pubkey()),
        //     &[&self.payer],
        //     self.rpc.get_latest_blockhash().unwrap(),
        // );

        // let sig = self.rpc.send_and_confirm_transaction(&tx).unwrap();
        // println!("✅ Agente inicializado: {}", sig);
        // Ok(pda)
        Ok("STUB".to_string())
    }
}

use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use serum_dex::{
    instruction::MarketInstruction,
    matching::{OrderType, Side},
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
    pubkey::Pubkey,
};

#[derive(Accounts)]
pub struct InitializeSwapPoolContext<'info> {
    #[account(init, payer = initializer, space = 8 + 16 + 16)]
    pub pool_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub initializer: Signer<'info>,
    pub system_program: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug)]
pub struct InitializeSwapPool {
    pub source_mint: Pubkey,
    pub target_mint: Pubkey,
    pub pool_fee: u8, // Fee taken on swaps (percentage)
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug)]
pub struct SwapInstruction {
    pub source_amount: u64, // Amount of source token (SOL) to swap
    pub min_target_amount: u64, // Minimum amount of target token (USDT) to receive
}

#[program]
mod solana_swap {
    use super::*;

    const MINT_DECIMALS: u8 = 6;

    pub fn initialize_swap_pool(ctx: Context<InitializeSwapPool>, data: InitializeSwapPool) -> ProgramResult {
        let pool_account = &mut ctx.accounts.pool_account;

        pool_account.mint.decimals = MINT_DECIMALS;
        pool_account.mint.pubkey = data.source_mint;

        if data.target_mint == Pubkey::default() {
            msg!("Error: Target mint pubkey not provided");
            return Err(ProgramError::InvalidArgument);
        }
        pool_account.target_mint = data.target_mint;

        if data.pool_fee > 100 {
            msg!("Error: Invalid pool fee. Fee must be less than or equal to 100%");
            return Err(ProgramError::InvalidArgument);
        }
        pool_account.pool_fee = data.pool_fee;

        Ok(())
    }

    pub fn swap(ctx: Context<SwapInstruction>, data: SwapInstruction) -> ProgramResult {
        let sol_amount = data.source_amount;

        let mut accounts_iter = &mut ctx.accounts.iter();

        let market_account = next_account_info(&mut accounts_iter)?;

      
        let source_account = next_account_info(&mut accounts_iter)?;
        let target_account = next_account_info(&mut accounts_iter)?;

        let instruction = MarketInstruction::NewOrderV3(
            serum_dex::instruction::NewOrderInstructionV3 {
                side: Side::Ask,
                limit_price: None, 
                max_coin_qty: sol_amount,
                order_type: OrderType::ImmediateOrCancel,
                client_id: 0, // Replace with unique client ID if needed
                self_trade_behavior: serum_dex::instruction::SelfTradeBehavior::DecrementTake,
                limit: None,
            },
        );

        serum_dex::instruction::market::new_order_v3(
            &ctx.accounts.program_id,
            &market_account.key,
            &instruction,
        )?;

        Ok(())
    }
}

use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use serum_dex::{
    instruction::MarketInstruction,
    matching::{OrderType, Side},
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
};

// Define our custom instruction types
#[derive(AnchorSerialize, AnchorDeserialize, Debug)]
pub struct InitializeSwapPool {
    pub source_mint: Pubkey,
    pub target_mint: Pubkey,
    pub pool_fee: u8, // Fee taken on swaps (percentage)
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug)]
pub struct SwapInstruction {
    pub source_amount: u64,      // Amount of source token (SOL) to swap
    pub min_target_amount: u64,  // Minimum amount of target token (USDT) to receive
}

#[program]
pub mod solana_swap {
    use super::*;

    // Define constants
    const SERUM_DEX_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
        0x42, 0x9e, 0x0d, 0x3b, 0x48, 0x3c, 0x90, 0x98, 0x40, 0x80, 0xf4, 0x2e, 0xcd, 0x57, 0xfa, 0x07,
        0xff, 0x5a, 0x1f, 0x5b, 0xc2, 0xdc, 0x67, 0x5b, 0x6a, 0x6a, 0x3a, 0xa9, 0xf9, 0x40, 0x06, 0x6e,
    ]);
    const MINT_DECIMALS: u8 = 6;

    // Initialize a new swap pool
    #[access_control(initialize_pool_control)]
    pub fn initialize_swap_pool(ctx: Context<InitializeSwapPool>, data: InitializeSwapPool) -> ProgramResult {
        let pool_account = &mut ctx.accounts.pool_account;

        // Set pool configuration based on data received
        pool_account.mint.decimals = MINT_DECIMALS;
        pool_account.mint.pubkey = data.source_mint;

        // Validate provided target mint
        if data.target_mint == Pubkey::default() {
            msg!("Error: Target mint pubkey not provided");
            return Err(ProgramError::InvalidArgument);
        }
        pool_account.target_mint = data.target_mint;

        // Validate pool fee
        if data.pool_fee > 100 {
            msg!("Error: Invalid pool fee. Fee must be less than or equal to 100%");
            return Err(ProgramError::InvalidArgument);
        }
        pool_account.pool_fee = data.pool_fee;

        Ok(())
    }

    // Perform a token swap (using Serum DEX)
    pub fn swap(ctx: Context<SwapInstruction>, data: SwapInstruction) -> ProgramResult {
        let sol_amount = data.source_amount;

        // Extract accounts from the provided slice
        let accounts_iter = &mut ctx.accounts.iter();

        // Get the serum DEX market account
        let market_account = next_account_info(accounts_iter)?;

        // Validate the market account owner
        if market_account.owner != &SERUM_DEX_PROGRAM_ID {
            msg!("Error: Invalid Serum DEX market account owner");
            return Err(ProgramError::IncorrectProgramId);
        }

        // Get user's source token (SOL) account and target token (USDT) account
        let source_account = next_account_info(accounts_iter)?;
        let target_account = next_account_info(accounts_iter)?;

        // Create a new instruction to swap SOL for USDT on the Serum DEX market
        let instruction = MarketInstruction::NewOrderV3(
            serum_dex::instruction::NewOrderInstructionV3 {
                side: Side::Ask,
                limit_price: None, // Set limit price based on fetched data
                max_coin_qty: sol_amount,
                order_type: OrderType::ImmediateOrCancel,
                client_id: 0, // Replace with unique client ID if needed
                self_trade_behavior: serum_dex::matching::SelfTradeBehavior::DecrementTake,
                limit: None,
            },
        );

        // Send the instruction to the Serum DEX market account
        serum_dex::instruction::market::new_order_v3(
            &ctx.accounts.program_id,
            &market_account.key,
            &instruction,
        )?;

        Ok(())
    }

    // Access control logic for initializing the pool
    fn initialize_pool_control(
        ctx: Context<InitializeSwapPool>,
        _data: &InitializeSwapPool,
    ) -> Result<()> {
        // Add access control logic for initializing the pool
        // For example, check if the caller is the expected initializer
        // Replace the condition below with your own access control logic
        if *ctx.accounts.initializer.key != ctx.accounts.pool_account.key {
            return Err(ProgramError::MissingRequiredSignature);
        }
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeSwapPoolContext<'info> {
    #[account(init, mint::decimals = MINT_DECIMALS, payer = initializer, space = 8 + 16 + 16)]
    pub pool_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub initializer: Signer<'info>,
    pub system_program: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
}

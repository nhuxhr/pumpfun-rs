pub mod utils;

use std::ops::{Add, Sub};
use std::rc::Rc;

use futures::future::try_join_all;
use pumpfun::utils::{get_mint_token_program, CreateTokenMetadata};
use pumpfun::{
    amm::PumpAmm,
    common::types::{SwapDirection, SwapInput},
};
use serial_test::serial;
use solana_sdk::{native_token::sol_str_to_lamports, signer::Signer};
use spl_associated_token_account::{
    get_associated_token_address, get_associated_token_address_with_program_id,
};
use spl_token::native_mint;
use tempfile::TempDir;
use utils::TestContext;

#[tokio::test]
#[serial]
async fn test_01_get_global_account() {
    let ctx = TestContext::default();
    let global_acct = ctx
        .client
        .get_global_account()
        .await
        .expect("Failed to get global account");

    assert!(
        global_acct.initialized,
        "Global account should be initialized"
    );
}

#[cfg(not(skip_expensive_tests))]
#[tokio::test]
#[serial]
async fn test_02_create_token() {
    if std::env::var("SKIP_EXPENSIVE_TESTS").is_ok() {
        return;
    }

    let ctx = TestContext::default();

    // Mint keypair
    let mint = ctx.mint.insecure_clone();

    // Check if the mint account already exists (optional, depending on your setup)
    if ctx.client.rpc.get_account(&mint.pubkey()).await.is_err() {
        // Use TempDir for temporary file management
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        let file_path = temp_dir.path().join("test_image.png");
        std::fs::write(&file_path, b"fake image data").expect("Failed to write temporary file");

        let metadata = CreateTokenMetadata {
            name: "Cat On Horse".to_string(),
            symbol: "COH".to_string(),
            description: "Lorem ipsum dolor, sit amet consectetur adipisicing elit.".to_string(),
            file: file_path.to_str().unwrap().to_string(),
            twitter: None,
            telegram: None,
            website: Some("https://example.com".to_string()),
        };

        let signature = ctx
            .client
            .create(mint.insecure_clone(), metadata.clone(), None)
            .await
            .expect("Failed to create token");
        println!("Signature: {}", signature);
        println!("{} Mint: {}", metadata.symbol, mint.pubkey());

        let curve = ctx
            .client
            .get_bonding_curve_account(&mint.pubkey())
            .await
            .expect("Failed to get bonding curve");
        println!("{} Bonding Curve: {:#?}", metadata.symbol, curve);
    } else {
        println!("Mint already exists: {}", mint.pubkey());
    }
}

#[cfg(not(skip_expensive_tests))]
#[tokio::test]
#[serial]
async fn test_03_buy_token() {
    if std::env::var("SKIP_EXPENSIVE_TESTS").is_ok() {
        return;
    }

    let ctx = TestContext::default();
    let mint = ctx.mint.pubkey();

    let signature = ctx
        .client
        .buy(mint, sol_str_to_lamports("1.0").unwrap(), None, None)
        .await
        .expect("Failed to buy tokens");
    println!("Signature: {}", signature);
}

#[cfg(not(skip_expensive_tests))]
#[tokio::test]
#[serial]
async fn test_04_sell_token() {
    if std::env::var("SKIP_EXPENSIVE_TESTS").is_ok() {
        return;
    }

    let ctx = TestContext::default();
    let mint = ctx.mint.pubkey();

    let signature = ctx
        .client
        .sell(mint, None, None, None)
        .await
        .expect("Failed to sell tokens");
    println!("Signature: {}", signature);
}

#[cfg(not(skip_expensive_tests))]
#[tokio::test]
#[serial]
async fn test_05_create_pool() {
    if std::env::var("SKIP_EXPENSIVE_TESTS").is_ok() {
        return;
    }

    let ctx = TestContext::default();
    let pool = PumpAmm::get_pool_pda(0, &ctx.payer.pubkey(), &ctx.mint.pubkey(), &native_mint::ID);

    // Check if the pool account already exists (optional, depending on your setup)
    if ctx.client.rpc.get_account(&pool).await.is_err() {
        ctx.client
            .buy(ctx.mint.pubkey(), sol_to_lamports(1f64), None, None)
            .await
            .expect("Failed to buy tokens");

        let ata = get_associated_token_address(&ctx.payer.pubkey(), &ctx.mint.pubkey());
        let balance = ctx
            .client
            .rpc
            .get_token_account_balance(&ata)
            .await
            .unwrap();

        let signature = ctx
            .client
            .amm
            .create_pool(
                0,
                ctx.mint.pubkey(),
                native_mint::ID,
                balance.amount.parse::<u64>().unwrap(),
                sol_to_lamports(10f64),
                None,
            )
            .await
            .expect("Failed to create pool");
        println!("Signature: {}", signature);
        println!("Pool: {}", pool);

        let pool_account = ctx
            .client
            .amm
            .get_pool_account(&pool)
            .await
            .expect("Failed to get pool account");
        println!("Pool Account: {:#?}", pool_account.1);
    } else {
        let pool_account = ctx
            .client
            .amm
            .get_pool_account(&pool)
            .await
            .expect("Failed to get pool account");

        println!("Pool already exists: {}", pool);
        println!("Pool Account: {:#?}", pool_account.1);
    }
}

#[cfg(not(skip_expensive_tests))]
#[tokio::test]
#[serial]
async fn test_06_deposit_lp() {
    if std::env::var("SKIP_EXPENSIVE_TESTS").is_ok() {
        return;
    }

    let ctx = TestContext::default();
    let pool = PumpAmm::get_pool_pda(0, &ctx.payer.pubkey(), &ctx.mint.pubkey(), &native_mint::ID);
    let pool_account = ctx
        .client
        .amm
        .get_pool_account(&pool)
        .await
        .expect("Failed to get pool account")
        .1;
    let lp_token = 100_000;

    ctx.client
        .buy(ctx.mint.pubkey(), sol_to_lamports(1f64), None, None)
        .await
        .expect("Failed to buy tokens");

    let signature = ctx
        .client
        .amm
        .deposit(pool, lp_token, 100, None)
        .await
        .expect("Failed to deposit LP");
    println!("Signature: {}", signature);

    let new_pool_account = ctx
        .client
        .amm
        .get_pool_account(&pool)
        .await
        .expect("Failed to get pool account")
        .1;
    println!("Pool Account: {:#?}", new_pool_account);

    assert_eq!(
        pool_account.lp_supply.add(lp_token),
        new_pool_account.lp_supply
    );

    ctx.client
        .sell(ctx.mint.pubkey(), None, None, None)
        .await
        .expect("Failed to sell tokens");
}

#[cfg(not(skip_expensive_tests))]
#[tokio::test]
#[serial]
async fn test_07_withdraw_lp() {
    if std::env::var("SKIP_EXPENSIVE_TESTS").is_ok() {
        return;
    }

    let ctx = TestContext::default();
    let pool = PumpAmm::get_pool_pda(0, &ctx.payer.pubkey(), &ctx.mint.pubkey(), &native_mint::ID);
    let pool_account = ctx
        .client
        .amm
        .get_pool_account(&pool)
        .await
        .expect("Failed to get pool account")
        .1;
    let lp_token = 100_000;

    let signature = ctx
        .client
        .amm
        .withdraw(pool, lp_token, 0, None)
        .await
        .expect("Failed to withdraw LP");
    println!("Signature: {}", signature);

    let new_pool_account = ctx
        .client
        .amm
        .get_pool_account(&pool)
        .await
        .expect("Failed to get pool account")
        .1;
    println!("Pool Account: {:#?}", new_pool_account);

    assert_eq!(
        pool_account.lp_supply.sub(lp_token),
        new_pool_account.lp_supply
    );

    ctx.client
        .sell(ctx.mint.pubkey(), None, None, None)
        .await
        .expect("Failed to sell tokens");
}

#[cfg(not(skip_expensive_tests))]
#[tokio::test]
#[serial]
async fn test_08_swap() {
    if std::env::var("SKIP_EXPENSIVE_TESTS").is_ok() {
        return;
    }

    let ctx = Rc::new(TestContext::default());
    let base_mint = ctx.mint.pubkey();
    let quote_mint = native_mint::ID;
    let pool = PumpAmm::get_pool_pda(0, &ctx.payer.pubkey(), &base_mint, &quote_mint);
    let global = ctx
        .client
        .amm
        .get_global_config_account()
        .await
        .expect("Failed to get global config")
        .1;

    // Ensure pool exists and has sufficient liquidity
    let (_, initial_base_balance, initial_quote_balance) = ctx
        .client
        .amm
        .get_pool_balances(&pool)
        .await
        .expect("Failed to get pool balances");
    assert!(
        initial_base_balance > 0,
        "Pool must have base token liquidity"
    );
    assert!(
        initial_quote_balance > 0,
        "Pool must have quote token liquidity"
    );

    let rpc = ctx.client.rpc.clone();
    let mint_token_programs = try_join_all(vec![
        get_mint_token_program(rpc.clone(), &base_mint),
        get_mint_token_program(rpc.clone(), &quote_mint),
    ])
    .await
    .expect("Failed to get mint token programs");
    let base_token_program = mint_token_programs[0];
    let quote_token_program = mint_token_programs[1];

    let get_user_balances = async |ctx: Rc<TestContext>| -> (u64, u64) {
        let user_base_token_account = get_associated_token_address_with_program_id(
            &ctx.payer.pubkey(),
            &base_mint,
            &base_token_program,
        );
        let user_quote_token_account = get_associated_token_address_with_program_id(
            &ctx.payer.pubkey(),
            &quote_mint,
            &quote_token_program,
        );

        let rpc = ctx.client.rpc.clone();
        let mint_token_balances = try_join_all({
            vec![
                rpc.get_token_account_balance(&user_base_token_account),
                rpc.get_token_account_balance(&user_quote_token_account),
            ]
        })
        .await
        .expect("Failed to get token balances");

        (
            mint_token_balances[0].amount.parse::<u64>().unwrap(),
            mint_token_balances[1].amount.parse::<u64>().unwrap(),
        )
    };

    // Initial token acquisition
    ctx.client
        .buy(ctx.mint.pubkey(), sol_to_lamports(0.01), None, None)
        .await
        .expect("Failed to buy initial tokens");

    // Test case 1: Base input buy
    {
        let balances = get_user_balances(ctx.clone()).await;
        let amount = 100_000;
        let slippage = 1;

        ctx.client
            .amm
            .swap(
                pool,
                amount,
                slippage,
                SwapInput::Base,
                SwapDirection::QuoteToBase,
                None,
            )
            .await
            .expect("Base input buy failed");

        let new_balances = get_user_balances(ctx.clone()).await;
        println!(
            "Base input buy - Previous: {:?}; New: {:?}",
            balances, new_balances
        );
        assert!(new_balances.0 > balances.0, "Base balance should increase");
        assert!(
            new_balances.1 <= balances.1,
            "Quote balance should decrease"
        );
    }

    // Test case 2: Base input sell
    {
        let balances = get_user_balances(ctx.clone()).await;
        let amount = 100_000;
        let slippage = 1;

        ctx.client
            .amm
            .swap(
                pool,
                amount,
                slippage,
                SwapInput::Base,
                SwapDirection::BaseToQuote,
                None,
            )
            .await
            .expect("Base input sell failed");

        let new_balances = get_user_balances(ctx.clone()).await;
        println!(
            "Base input sell - Previous: {:?}; New: {:?}",
            balances, new_balances
        );
        assert_eq!(
            balances.0.sub(amount),
            new_balances.0,
            "Base balance decrease should match amount"
        );
        assert!(new_balances.1 > balances.1, "Quote balance should increase");
    }

    // Test case 3: Quote input buy
    {
        let (_, pool_base_balance, pool_quote_balance) = ctx
            .client
            .amm
            .get_pool_balances(&pool)
            .await
            .expect("Failed to get pool balances");

        let balances = get_user_balances(ctx.clone()).await;
        let amount = 100_000;
        let slippage = 1;

        let (_, expected_base, _) = pumpfun::utils::amm::buy::buy_quote_input(
            amount,
            slippage,
            pool_base_balance,
            pool_quote_balance,
            global.lp_fee_basis_points,
            global.protocol_fee_basis_points,
        )
        .expect("Failed to calculate expected base amount");

        ctx.client
            .amm
            .swap(
                pool,
                amount,
                slippage,
                SwapInput::Quote,
                SwapDirection::QuoteToBase,
                None,
            )
            .await
            .expect("Quote input buy failed");

        let new_balances = get_user_balances(ctx.clone()).await;
        println!(
            "Quote input buy - Previous: {:?}; New: {:?}",
            balances, new_balances
        );
        assert_eq!(
            balances.0.add(expected_base),
            new_balances.0,
            "Base increase should match expected"
        );
    }

    // Test case 4: Quote input sell
    {
        let (_, pool_base_balance, pool_quote_balance) = ctx
            .client
            .amm
            .get_pool_balances(&pool)
            .await
            .expect("Failed to get pool balances");

        let balances = get_user_balances(ctx.clone()).await;
        let amount = 100_000;
        let slippage = 1;

        let (_, expected_base, _) = pumpfun::utils::amm::sell::sell_quote_input(
            amount,
            slippage,
            pool_base_balance,
            pool_quote_balance,
            global.lp_fee_basis_points,
            global.protocol_fee_basis_points,
        )
        .expect("Failed to calculate expected base amount");

        ctx.client
            .amm
            .swap(
                pool,
                amount,
                slippage,
                SwapInput::Quote,
                SwapDirection::BaseToQuote,
                None,
            )
            .await
            .expect("Quote input sell failed");

        let new_balances = get_user_balances(ctx.clone()).await;
        println!(
            "Quote input sell - Previous: {:?}; New: {:?}",
            balances, new_balances
        );
        assert_eq!(
            balances.0.sub(expected_base),
            new_balances.0,
            "Base decrease should match expected"
        );
    }

    // Cleanup
    ctx.client
        .sell(ctx.mint.pubkey(), None, None, None)
        .await
        .expect("Failed to sell remaining tokens");
}

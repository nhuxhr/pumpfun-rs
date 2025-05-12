pub mod utils;

use std::ops::Add;

use pumpfun::amm::PumpAmm;
use pumpfun::utils::CreateTokenMetadata;
use serial_test::serial;
use solana_sdk::{native_token::sol_str_to_lamports, signer::Signer};
use spl_associated_token_account::get_associated_token_address;
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

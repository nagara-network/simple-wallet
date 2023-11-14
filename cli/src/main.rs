#[derive(clap::Parser, core::fmt::Debug)]
#[clap(author, version, about)]
enum Commands {
    Check {
        /// SS58 address to check
        #[clap(short, long)]
        account: String,
    },
    Transfer {
        /// Sender private key hex (starts with "0x"), can also be mnemonic. Always surround it with ""
        #[clap(short, long, value_parser)]
        private_key: String,
        /// Recipient's SS58 address
        #[clap(short, long, value_parser)]
        recipient: String,
        /// NGR Amount in decimal
        #[clap(short, long, value_parser)]
        amount: bigdecimal::BigDecimal,
        /// Sender use ed25519 instead of sr25519
        #[clap(short, long, action, default_value_t = false)]
        edward: bool,
    },
}

fn get_decimal_scaler() -> bigdecimal::BigDecimal {
    let decimal_place =
        ss58_registry::Token::from(ss58_registry::TokenRegistry::Ngr).decimals as u32;

    <bigdecimal::BigDecimal as bigdecimal::FromPrimitive>::from_i64(10i64.pow(decimal_place))
        .unwrap()
}

impl Commands {
    async fn run() -> anyhow::Result<()> {
        nagara_logging::init();

        let command = <Self as clap::Parser>::parse();
        let mut instance = nagara_simple_wallet::WalletInstance::create_with_default_url().await?;

        match command {
            Self::Check { account } => {
                let balance = instance.check_balance(&account).await?;
                let balance_decimal =
                    <bigdecimal::BigDecimal as bigdecimal::FromPrimitive>::from_u128(balance)
                        .unwrap();
                let balance_decimal = std::ops::Div::div(balance_decimal, get_decimal_scaler());

                nagara_logging::info!("Balance is:\n\n{balance_decimal} NGR");
            }
            Self::Transfer {
                private_key,
                recipient,
                amount,
                edward,
            } => {
                let schnorrkel = !edward;
                let sender_address = instance.add_account(&private_key, schnorrkel)?;
                nagara_logging::info!(
                    "Sending from {sender_address} to {recipient} with the amount of {amount} ({})",
                    if schnorrkel { "sr25519" } else { "ed25519" }
                );
                let amount_decimal = std::ops::Mul::mul(amount, get_decimal_scaler());
                let amount = bigdecimal::ToPrimitive::to_u128(&amount_decimal)
                    .ok_or(anyhow::anyhow!("Bad digits!"))?;
                let explorer_url = instance
                    .transfer(&sender_address, &recipient, amount)
                    .await?;

                nagara_logging::info!("Transaction was successful, info:\n\n{explorer_url}");
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Commands::run().await
}

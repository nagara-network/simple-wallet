#[cfg(all(feature = "default", feature = "wasm32"))]
compile_error!("Feature \"default\" can't be combined with \"wasm32\".");

pub(crate) mod metadata;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(core::fmt::Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    SubxtError(#[from] subxt::Error),
    #[error("{0}")]
    OtherError(String),
    #[error("Account not found")]
    AccountNotFound,
    #[error("Maximum account custody reached")]
    AccountFull,
}

impl From<crate::Error> for i32 {
    fn from(value: crate::Error) -> Self {
        match value {
            crate::Error::SubxtError(_) => -1,
            crate::Error::OtherError(_) => -2,
            crate::Error::AccountNotFound => -3,
            crate::Error::AccountFull => -4,
        }
    }
}

pub struct OwnedAccount {
    identity: nagara_identities::CryptographicIdentity,
    use_schnorrkel: bool,
}

impl OwnedAccount {
    pub fn new_from_str(secret_str: &str, use_schnorrkel: bool) -> crate::Result<Self> {
        let identity = nagara_identities::CryptographicIdentity::try_from_private_str(secret_str)
            .map_err(|err| Error::OtherError(err.to_string()))?;

        Ok(Self {
            identity,
            use_schnorrkel,
        })
    }

    pub fn get_main_address(&self) -> String {
        if self.use_schnorrkel {
            self.identity
                .try_get_public_sr25519()
                .unwrap()
                .get_main_address()
                .to_string()
        } else {
            self.identity
                .try_get_public_ed25519()
                .unwrap()
                .get_main_address()
                .to_string()
        }
    }

    pub fn get_storage_address(&self) -> String {
        if self.use_schnorrkel {
            self.identity
                .try_get_public_sr25519()
                .unwrap()
                .get_storage_address()
                .to_string()
        } else {
            self.identity
                .try_get_public_ed25519()
                .unwrap()
                .get_storage_address()
                .to_string()
        }
    }
}

impl subxt::tx::Signer<subxt::PolkadotConfig> for OwnedAccount {
    fn account_id(&self) -> subxt::utils::AccountId32 {
        let identity_ss58 = if self.use_schnorrkel {
            self.identity
                .try_get_public_sr25519()
                .map(|pubkey| pubkey.get_main_address())
                .unwrap()
        } else {
            self.identity
                .try_get_public_ed25519()
                .map(|pubkey| pubkey.get_main_address())
                .unwrap()
        };

        <subxt::utils::AccountId32 as core::str::FromStr>::from_str(&identity_ss58).unwrap()
    }

    fn address(&self) -> <subxt::PolkadotConfig as subxt::Config>::Address {
        <Self as subxt::tx::Signer<subxt::PolkadotConfig>>::account_id(self).into()
    }

    fn sign(&self, signer_payload: &[u8]) -> subxt::utils::MultiSignature {
        let signature = self
            .identity
            .try_sign(self.use_schnorrkel, signer_payload)
            .unwrap();

        if self.use_schnorrkel {
            subxt::utils::MultiSignature::Sr25519(signature)
        } else {
            subxt::utils::MultiSignature::Ed25519(signature)
        }
    }
}

pub struct WalletInstance {
    client: subxt::OnlineClient<subxt::PolkadotConfig>,
    accounts: std::collections::HashMap<String, OwnedAccount>,
}

impl WalletInstance {
    #[cfg(not(feature = "wasm32"))]
    pub const BOOTNODE_URL: &'static str = "wss://boot.nagara.network:443";
    #[cfg(feature = "wasm32")]
    pub const BOOTNODE_URL: &'static str = "https://boot.nagara.network:443";
    pub const BASE_BLOCK_URL: &'static str =
        "https://nagara.network/?rpc=wss%3A%2F%2Fboot.nagara.network#/explorer/query";
    pub const MAX_CUSTODY: usize = u8::MAX as usize;

    pub async fn create_with_default_url() -> crate::Result<Self> {
        Self::create_with_url(Self::BOOTNODE_URL).await
    }

    pub async fn create_with_url<U: core::convert::AsRef<str>>(url: U) -> crate::Result<Self> {
        let client = subxt::OnlineClient::<subxt::PolkadotConfig>::from_url(url).await?;

        Ok(Self {
            client,
            accounts: std::collections::HashMap::with_capacity(Self::MAX_CUSTODY),
        })
    }

    pub fn add_account(&mut self, secret_str: &str, use_schnorrkel: bool) -> crate::Result<String> {
        if self.accounts.len() >= Self::MAX_CUSTODY {
            return crate::Result::Err(Error::AccountFull);
        }

        let account = OwnedAccount::new_from_str(secret_str, use_schnorrkel)?;
        let account_str = account.get_main_address();
        self.accounts.insert(account_str.clone(), account);

        Ok(account_str)
    }

    pub async fn check_balance(&self, account_address: &str) -> crate::Result<u128> {
        let account =
            <subxt::utils::AccountId32 as core::str::FromStr>::from_str(account_address).unwrap();
        let data_pointer = metadata::nagara::api::storage().system().account(account);
        let maybe_account_info_exist = self
            .client
            .storage()
            .at_latest()
            .await?
            .fetch(&data_pointer)
            .await?;

        if let Some(account_info) = maybe_account_info_exist {
            let free_amount = account_info.data.free;
            let locked_amount = account_info.data.frozen;
            let reserved_amount = account_info.data.reserved;

            Ok(free_amount + locked_amount + reserved_amount)
        } else {
            Ok(0)
        }
    }

    pub async fn transfer(
        &self,
        sender_address: &str,
        recipient_address: &str,
        balance: u128,
    ) -> crate::Result<String> {
        let sender_account = self
            .accounts
            .get(sender_address)
            .ok_or(Error::AccountNotFound)?;
        let recipient_account =
            <subxt::utils::AccountId32 as core::str::FromStr>::from_str(recipient_address).unwrap();
        let tx_payload = metadata::nagara::api::tx()
            .balances()
            .transfer_keep_alive(recipient_account.into(), balance);
        let block_hash = self
            .client
            .tx()
            .sign_and_submit_then_watch_default(&tx_payload, sender_account)
            .await?
            .wait_for_in_block()
            .await?
            .block_hash();
        let block_hash_hex = hex::encode(block_hash);
        let block_info = format!("{}/0x{block_hash_hex}", Self::BASE_BLOCK_URL);

        Ok(block_info)
    }

    pub async fn latest_block(&self, finalized: bool) -> crate::Result<u32> {
        let blocks_client = self.client.blocks();
        let result = if finalized {
            blocks_client.subscribe_finalized().await?.next().await
        } else {
            blocks_client.subscribe_best().await?.next().await
        };

        match result {
            Some(block_subscribe_result) => {
                let block_info = block_subscribe_result?;

                Ok(block_info.number())
            }
            None => Ok(0),
        }
    }
}

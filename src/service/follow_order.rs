use crate::{Config, FollowsAddressConfig};
use ethers::{
    contract::{abigen, Contract},
    prelude::SignerMiddleware,
    providers::{Http, Middleware, Provider, StreamExt, Ws},
    signers::LocalWallet,
    types::{Address, TransactionRequest, H160, U256},
    utils::format_ether,
};
use std::{error::Error, ops::Deref, sync::Arc};
use tracing::info;
pub struct FollowOrder {}

abigen!(
    BuySharesContract,
    r#"[
        function buyShares(address sharesSubject, uint256 amount) public payable
        function getBuyPriceAfterFee(address sharesSubject, uint256 amount) public view returns (uint256)
    ]"#,
);

abigen!(
    TradeEvent,
    r#"[
        event Trade(address trader, address subject, bool isBuy, uint256 shareAmount, uint256 ethAmount, uint256 protocolEthAmount, uint256 subjectEthAmount, uint256 supply)
    ]"#,
);

impl FollowOrder {
    pub async fn do_follow_order(config: &Config) -> Result<(), Box<dyn Error>> {
        // tracing_subscriber::fmt().with_max_level(Level::INFO).init();
        info!("***** do_follow_order starting *****");
        let ws_provider = get_ws(config.clone()).await;
        let https_provider = get_http(config.clone()).await;
        // signer
        let signer = config
            .clone()
            .account
            .private_key
            .parse::<LocalWallet>()
            .unwrap();
        let provider_with_signer = Arc::new(
            SignerMiddleware::new_with_provider_chain(https_provider.clone(), signer)
                .await
                .unwrap(),
        );

        let chain_id = Arc::new(config.base_mainnet.chain_id);

        // friend 合约
        let contact_address = "0xCF205808Ed36593aa40a44F10c7f7C2F67d4A4d4"
            .parse::<Address>()
            .unwrap();

        let friend_contract = Arc::new(BuySharesContract::new(
            contact_address,
            https_provider.into(),
        ));
        let target_address_vec: Vec<String> = config
            .follows_address_config
            .iter()
            .map(|follows_config| follows_config.address.clone().to_lowercase())
            .collect();

        let event = Contract::event_of_type::<TradeFilter>(ws_provider.into());
        let mut stream = event.subscribe_with_meta().await.unwrap();
        loop {
            match stream.next().await {
                Some(Ok((log, _))) => {
                    let p = Arc::clone(&provider_with_signer);
                    let c = Arc::clone(&friend_contract);
                    let chain_id = Arc::clone(&chain_id);
                    let subject: H160 = log.subject;
                    let subject = Arc::new(subject);
                    let trader: H160 = log.trader;
                    let trader_str = format!("{trader:#010x}");
                    if target_address_vec.contains(&trader_str.to_lowercase()) && log.is_buy {
                        info!("***** 开始处理跟单 *****");
                        let matching_configs: Vec<&FollowsAddressConfig> = config
                            .follows_address_config
                            .iter()
                            .filter(|f| {
                                f.address.to_lowercase() == trader_str.to_string().to_lowercase()
                            })
                            .collect();

                        let follow_info = *matching_configs.get(0).unwrap();
                        let follow_info_arc = Arc::new(follow_info.clone());
                        tokio::spawn(async move {
                            handle_buy(chain_id, p, c, subject, Arc::clone(&follow_info_arc)).await;
                        });
                    }
                }
                Some(Err(e)) => {
                    info!("***** log流异常: {} *****", e)
                }
                None => {
                    info!("***** log流None *****");
                    continue;
                }
            }
        }
    }
}

async fn handle_buy(
    chain_id: Arc<u32>,
    provider_with_signer: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    contract: Arc<BuySharesContract<Provider<Http>>>,
    subject: Arc<H160>,
    follow_info_arc: Arc<FollowsAddressConfig>,
) {
    let chain_id = chain_id.to_string().parse::<u32>().unwrap();
    let subject = subject.deref();
    let subject = format!("{subject:#010x}").parse::<Address>().unwrap();
    let price_after_fee = contract
        .get_buy_price_after_fee(subject, U256::from(follow_info_arc.amount))
        .call()
        .await
        .unwrap();
    let price_after_fee_f32 = format_ether(price_after_fee).parse::<f32>().unwrap();
    if price_after_fee_f32 > follow_info_arc.balance {
        info!(
            "***** 当前key: {:?} 价格{}e 大于跟单设定上限金额{}e,不进行跟单操作 *****",
            subject, price_after_fee_f32, follow_info_arc.balance
        );
    } else {
        let tx_raw = TransactionRequest::new()
            .chain_id(chain_id)
            .to(contract.address())
            .data(
                contract
                    .buy_shares(subject, U256::from(follow_info_arc.amount))
                    .calldata()
                    .unwrap(),
            )
            .value(price_after_fee)
            .gas(90000)
            .gas_price(200000000);

        let pending_res = provider_with_signer
            .send_transaction(tx_raw.clone(), None)
            .await;
        match pending_res {
            Ok(t) => {
                info!(
                    "***** 跟单交易已发送: {:?},跟单人地址: {:?},购买key: {:?},购买价格: {}e *****",
                    t.tx_hash(),
                    follow_info_arc.address,
                    subject,
                    price_after_fee_f32
                );
            }
            Err(e) => {
                info!("***** 发送跟单交易失败:{:?}", e);
            }
        }
    }
}

// 获取ws端点
async fn get_ws(config: Config) -> Provider<Ws> {
    Provider::<Ws>::connect_with_reconnects(config.base_mainnet.ws, usize::MAX)
        .await
        .unwrap()
}

// 获取http端点
async fn get_http(config: Config) -> Provider<Http> {
    Provider::<Http>::try_from(config.base_mainnet.https).unwrap()
}

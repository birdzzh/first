mod config;
mod service;
pub use config::*;
pub use service::*;
use tracing::{info, Level};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
    let config = Config::load("YOUR_PATH");
    match config {
        Ok(c) => {
            info!("***** 加载配置文件success *****");
            let res = FollowOrder::do_follow_order(&c).await;
            match res {
                Err(e) => {
                    info!("***** 处理跟单异常: {:?} *****", e)
                }
                _ => {}
            }
        }
        Err(e) => info!("***** 加载配置文件failed: {:?}*****", e),
    }
}

use async_trait::async_trait;
use max_core::errors::Result;
use max_core::traits::ChannelAdapter;

pub struct CliChannel;

#[async_trait]
impl ChannelAdapter for CliChannel {
    fn name(&self) -> &str {
        "cli"
    }

    async fn send_message(&self, _recipient: &str, message: &str) -> Result<()> {
        println!("{}", message);
        Ok(())
    }
}

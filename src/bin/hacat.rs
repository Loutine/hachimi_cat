use std::str::FromStr;

use clap::{Parser, Subcommand};
use hachimi_cat::AudioServices;
use iroh::{Endpoint, EndpointId};

#[derive(Parser)]
#[command(name = "hacat")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Listen,
    Call { id: String },
}

const ALPN: &[u8] = b"hacat/opus/1";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mdns = iroh::discovery::mdns::MdnsDiscovery::builder();
    let dht = iroh::discovery::pkarr::dht::DhtDiscovery::builder();

    let alpns = vec![ALPN.to_vec()];

    let mut audio_services = AudioServices::new()?;

    match cli.command {
        Commands::Listen => {
            let endpoint = Endpoint::builder()
                .discovery(mdns)
                .discovery(dht)
                .alpns(alpns)
                .bind()
                .await?;
            let local_id = endpoint.id();
            println!("local id: {}", local_id);

            while let Some(incoming) = endpoint.accept().await {
                let connecting = incoming.accept()?;
                let connection = connecting.await?;

                audio_services.add_connection(connection)?;
            }
        }
        Commands::Call { id } => {
            let endpoint = Endpoint::builder()
                .discovery(mdns)
                .discovery(dht)
                .alpns(alpns)
                .bind()
                .await?;
            let connection = endpoint.connect(EndpointId::from_str(&id)?, ALPN).await?;

            audio_services.add_connection(connection)?;
        }
    }

    tokio::signal::ctrl_c().await?;

    // for service in running_services {
    // TODO: safety close connection
    // service.connection.close()
    // }

    println!("Shutting down.");
    Ok(())
}

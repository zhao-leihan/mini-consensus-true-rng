use clap::{Parser, Subcommand};
use api::start_server;

#[derive(Parser)]
#[command(name = "mini-consensus-node")]
#[command(about = "Mini consensus node with true RNG")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    #[arg(long, default_value_t = 8080)]
    port: u16,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the node server
    Server,
    /// Generate random bytes
    Rng {
        #[arg(default_value_t = 32)]
        len: usize,
    },
    /// Run TRNG health checks
    HealthCheck,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Server) => {
            println!("Starting mini-consensus node on port {}", cli.port);
            start_server(cli.port).await;
        }
        Some(Commands::Rng { len }) => {
            let trng = trng::Trng::new();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            let random_bytes = trng.rand_bytes(len);
            println!("{}", hex::encode(random_bytes));
        }
        Some(Commands::HealthCheck) => {
            let trng = trng::Trng::new();
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            
            let health = trng.health_check(65536); // 64KB sample
            
            println!("TRNG Health Check Results ({} bytes sample):", health.sample_size);
            println!("=============================================");
            println!("Monobit Test Deviation: {:.6} (should be < 0.01)", health.monobit_deviation);
            println!("Runs Test Deviation: {:.6} (should be < 0.1)", health.runs_deviation);
            println!("Shannon Entropy: {:.6} bits/byte (should be > 7.5)", health.shannon_entropy);
            println!("Overall Healthy: {}", health.is_healthy());
            
            // Negative control demonstration
            println!("\nNegative Control (Constant Pattern):");
            println!("====================================");
            let constant_data = vec![0x55u8; 8192];
            let monobit_dev = trng.monobit_test(&constant_data);
            let runs_dev = trng.runs_test(&constant_data);
            let entropy = trng.approximate_entropy(&constant_data);
            println!("Monobit Deviation: {:.6}", monobit_dev);
            println!("Runs Deviation: {:.6}", runs_dev);
            println!("Shannon Entropy: {:.6}", entropy);
        }
        None => {
            // Default to server mode
            println!("Starting mini-consensus node on port {}", cli.port);
            start_server(cli.port).await;
        }
    }
}
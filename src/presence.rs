use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DiscordIpcClient::new("1569148102085185808");

    client.connect()?;
    client.set_activity(activity::Activity::new().details("ReDox").state("Vibing"))?;

    println!("Activity set! Press Enter to clear it and exit...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    println!("Clearing activity...");
    client.clear_activity()?;

    println!("Activity cleared! Going to exit...");
    match client.close() {
        Ok(()) => (),
        Err(err) => println!("Error closing IPC connection: {}", err),
    }

    Ok(())
}

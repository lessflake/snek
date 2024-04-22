use snek::{
    cache::{Cache, Log},
    core, get_log_dir,
    sender::Webhook,
};

#[derive(Debug)]
pub enum Mode {
    Watch(Webhook),
    Daily(Webhook),
    Links,
}

#[tokio::main]
async fn main() {
    flexi_logger::Logger::with_str("info, snek = trace")
        .format(flexi_logger::default_format)
        .start()
        .unwrap();

    let mode = parse_args();

    let log_dir = match get_log_dir() {
        Ok(log_dir) => log_dir,
        Err(_) => {
            log::error!("log directory not found: as a backup option, make a file called `logdir.txt` in the same directory as snek and enter the path of your log directory in it i.e. `C:\\Users\\foobar\\Documents\\Guild Wars 2\\addons\\arcdps\\arcdps.cbtlogs`");
            std::process::exit(1);
        }
    };

    let cache = Log::new("log_cache").await.unwrap();
    let url = "https://dps.report/";

    let result = match mode {
        Mode::Daily(hook) => core::daily(log_dir, url, hook, cache).await,
        Mode::Links => core::links(log_dir, url, std::io::stdout(), cache).await,
        Mode::Watch(hook) => core::watch(log_dir, url, hook, cache).await,
    };

    match result {
        Ok(()) => {}
        Err(e) => {
            log::error!("error: {}", e);
        }
    }
}

fn parse_args() -> Mode {
    use std::env;

    let mut args = env::args();
    args.next().unwrap();

    let mode_name = args.next().unwrap_or_else(|| usage());

    match mode_name.as_ref() {
        "daily" => {
            let hook = parse_webhook_args(&mut args);
            return Mode::Daily(hook);
        }
        "links" => return Mode::Links,
        "watch" => {
            let hook = parse_webhook_args(&mut args);
            return Mode::Watch(hook);
        }

        "add" => add_webhook(&mut args),
        "remove" => remove_webhook(&mut args),
        "list" => list_webhooks(),

        "about" => about(),

        _ => {
            eprintln!("invalid mode: try `./snek` for usage");
            std::process::exit(1);
        }
    };

    std::process::exit(0);
}

fn usage() -> ! {
    println!("\
usage:
    ./snek watch <name>
        Watch for incoming fractal CM logs, upload & post to webhook <name> as
        embed

    ./snek daily <name>
        Upload & post set of most recent fractal CM logs to webhook <name>

    ./snek links
        Upload set of recent fractal CM logs & output as plaintext

    ./snek add <name> <url>
        Add a webhook: <name> is used to reference the webhook in other
        commands, <url> is a Discord webhook url

    ./snek remove <name>
        Forget the webhook <name>

    ./snek list
        List known webhooks

    ./snek about
        Version, background information and whatever

For example, initial setup:
> ./snek add my_webhook https://discordapp.com/api/webhooks/ABCDEFGHIJKLMNOPQR/ABCDEFGHIJKLMNOPQRSTUVWXYZ01234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ01234
> ./snek watch my_webhook

snek checks the default arcdps log directory and if it can't find anything
then it looks for a file in the same directory as it called `logdir.txt` which
is assumed to contain the user's actual arcdps log directory (for example,
C:\\Users\\you\\Documents\\Guild Wars 2\\addons\\arcdps\\arcdps.cbtlogs) ");
    std::process::exit(0)
}

fn parse_webhook_args(args: &mut std::env::Args) -> Webhook {
    let webhooks: Cache<String, String> =
        Cache::new_blocking("webhooks").expect("failed to read webhook store");

    let hook_name = args.next().unwrap_or_else(|| {
        eprintln!("invalid arguments: missing webhook name");
        std::process::exit(1);
    });

    let hook_url = webhooks.get(&hook_name).unwrap_or_else(|| {
        eprintln!("no such webhook: try adding with `./snek add <name> <url>`");
        std::process::exit(1);
    });

    Webhook::new(&hook_url)
}

fn add_webhook(args: &mut std::env::Args) {
    let mut webhooks: Cache<String, String> =
        Cache::new_blocking("webhooks").expect("failed to read webhook store");

    if let (Some(hook_name), Some(hook_url)) = (args.next(), args.next()) {
        if !Webhook::validate_url(&hook_url) {
            eprintln!("\
error: ./snek add <name> <url>
    provided url is incorrect format, should be copied directly from discord
    i.e. ./snek add my_webhook https://discordapp.com/api/webhooks/ABCDEFGHIJKLMNOPQR/ABCDEFGHIJKLMNOPQRSTUVWXYZ01234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ01234");
            std::process::exit(1);
        }

        webhooks.insert(hook_name.clone(), hook_url);

        log::info!("added webhook `{}`", hook_name);
    } else {
        eprintln!("invalid arguments: try `./snek add <name> <url>`");
        std::process::exit(1);
    }
}

fn remove_webhook(args: &mut std::env::Args) {
    let mut webhooks: Cache<String, String> =
        Cache::new_blocking("webhooks").expect("failed to read webhook store");

    if let Some(hook_name) = args.next() {
        match webhooks.remove(&hook_name) {
            Some(_) => log::info!("removed webhook `{}`", &hook_name),
            None => log::error!("`{}` is not a known webhook", &hook_name),
        };
    } else {
        eprintln!("invalid arguments: try `./snek remove <name>`");
        std::process::exit(1);
    }
}

fn list_webhooks() {
    let webhooks: Cache<String, String> =
        Cache::new_blocking("webhooks").expect("failed to read webhook store");

    log::info!("listing known webhooks");
    for (k, v) in webhooks.raw() {
        println!("{}: {}", k, v);
    }

    if webhooks.raw().is_empty() {
        println!("no known webhooks");
    }
}

fn about() -> ! {
    println!(
        "  v1.0.4 20201031

snek is an automatic GW2 log uploader, specifically for Fractal CM logs.  To
keep it simple, all other bosses are explicitly unsupported.

Log files are automatically parsed for necessary information.  Some particular
logs may be problematic and display incorrect details.  If you know me, feel
free to send me the .[z]evtc files for these logs and I'll fix the issue
(eventually).

20201031: added support for Ai, Keeper of the Peak; fixed several phase
          parsing errors
"
    );

    std::process::exit(0)
}

mod interception;
mod simple;

pub use interception::{BangCommandInterception, BangCommandResponse, fn_command};

pub async fn initialize_bang_command() -> BangCommandInterception {
    let interception = BangCommandInterception::new();
    interception.register_command("ping", fn_command(simple::ping)).await;
    interception
        .register_command("change", fn_command(simple::change))
        .await;

    interception
}

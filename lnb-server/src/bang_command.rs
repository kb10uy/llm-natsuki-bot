mod interception;
mod simple;

use self::{
    interception::{BangCommandInterception, fn_command},
    simple::ping,
};

pub async fn initialize_bang_command() -> BangCommandInterception {
    let interception = BangCommandInterception::new();
    interception.register_command("ping", fn_command(ping)).await;

    interception
}

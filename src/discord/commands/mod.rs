pub mod admin_role_map;
pub mod admin_role_tier;
pub mod ban;
pub mod help;
pub mod history;
pub mod lobby;
pub mod lobby_add;
pub mod lobby_balance;
pub mod lobby_close;
pub mod lobby_create;
pub mod lobby_join;
pub mod lobby_leave;
pub mod lobby_list;
pub mod lobby_remove;
pub mod lobby_seed;
pub(crate) mod moderation_common;
pub mod register;
pub mod unban;
pub mod warn;
pub mod whoishere;

use serenity::all::CreateCommand;

pub fn all() -> Vec<CreateCommand> {
    vec![
        help::register(),
        whoishere::register(),
        register::register(),
        lobby::register(),
        lobby_balance::register(),
        lobby_create::register(),
        lobby_list::register(),
        lobby_close::register(),
        lobby_join::register(),
        lobby_leave::register(),
        lobby_add::register(),
        lobby_remove::register(),
        lobby_seed::register(),
        admin_role_tier::register(),
        admin_role_map::register(),
        warn::register(),
        ban::register(),
        unban::register(),
        history::register(),
    ]
}

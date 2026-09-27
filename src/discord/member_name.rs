use serenity::all::{Context, GuildId, Member, User, UserId};

pub(crate) fn interaction_name(user: &User, member: Option<&Member>) -> String {
    member
        .map(Member::display_name)
        .unwrap_or_else(|| user.display_name())
        .to_owned()
}

pub(crate) async fn guild_display_name(
    ctx: &Context,
    guild: GuildId,
    discord_id: &str,
    fallback: &str,
) -> String {
    let Ok(user_id) = discord_id.parse::<u64>().map(UserId::new) else {
        return fallback.to_owned();
    };

    guild
        .member(ctx, user_id)
        .await
        .map(|member| member.display_name().to_owned())
        .unwrap_or_else(|_| fallback.to_owned())
}

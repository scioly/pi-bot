use crate::discord::{Context, Error};

static ROLE_STAFF: &str = "Staff";
static ROLE_VIP: &str = "VIP";

pub async fn is_staff(ctx: Context<'_>) -> Result<bool, Error> {
    let guild_roles = if let Some(guild) = ctx.guild() {
        let mut roles = guild.roles.values().cloned().collect::<Box<[_]>>();
        roles.sort();
        roles
    } else {
        // TODO: error
        return Ok(false);
    };

    let staff_role = guild_roles.iter().find(|&role| role.name == ROLE_STAFF);
    let vip_role = guild_roles.iter().find(|&role| role.name == ROLE_VIP);

    let authorized_roles = [staff_role, vip_role]
        .into_iter()
        .flatten()
        .collect::<Box<[_]>>();

    if let Some(member) = ctx.author_member().await {
        for &user_role_id in member.roles.iter() {
            for expected_role in authorized_roles.iter() {
                if expected_role.id == user_role_id {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

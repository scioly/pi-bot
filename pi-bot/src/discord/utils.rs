use poise::serenity_prelude::Member;

use crate::discord::{Context, Error};

static ROLE_STAFF: &str = "Staff";
static ROLE_VIP: &str = "VIP";

pub static EMOJI_LOADING: &str = "<a:loading:1409087568313712731>";

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

#[derive(Debug, PartialEq, Eq)]
pub enum Pronoun {
    He,
    She,
    They,
}

impl Pronoun {
    /// Returns either "himself", "herself", or "themself" based on the pronoun.
    pub fn get_pronounself(&self) -> &str {
        match self {
            Pronoun::He => "himself",
            Pronoun::She => "herself",
            Pronoun::They => "themself",
        }
    }
}

const ROLE_PRONOUN_HE: &str = "He / Him / His";
const ROLE_PRONOUN_SHE: &str = "She / Her / Hers";

/// Figures out what pronouns to use for the given user.
///
/// If no pronouns are selected, then [`Pronoun::They`] will be used. If conflicting pronouns are
/// set (i.e if [`Pronoun::He`] and [`Pronoun::She`] are set), then [`Pronoun::They`] will also be
/// used.
pub fn determine_pronouns(ctx: Context<'_>, member: &Member) -> Pronoun {
    const DEFAULT_PRONOUN: Pronoun = Pronoun::They;
    let roles = match member.roles(ctx.cache()) {
        Some(roles) => roles,
        None => return DEFAULT_PRONOUN,
    };

    roles
        .iter()
        .filter_map(|role| match role.name.as_str() {
            ROLE_PRONOUN_HE => Some(Pronoun::He),
            ROLE_PRONOUN_SHE => Some(Pronoun::She),
            // ROLE_PRONOUN_THEY => Some(Pronoun::They),
            _ => None,
        })
        .fold(None, |acc, pronoun| match acc {
            None => match pronoun {
                Pronoun::She | Pronoun::He => Some(pronoun),
                _ => None,
            },
            Some(acc) => {
                if acc == pronoun || matches!(pronoun, Pronoun::They) {
                    Some(acc)
                } else {
                    Some(DEFAULT_PRONOUN)
                }
            }
        })
        .unwrap_or(DEFAULT_PRONOUN)
}

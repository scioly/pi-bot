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
pub enum BinaryPronoun {
    He,
    She,
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

impl From<BinaryPronoun> for Pronoun {
    fn from(value: BinaryPronoun) -> Self {
        match value {
            BinaryPronoun::He => Pronoun::He,
            BinaryPronoun::She => Pronoun::She,
        }
    }
}

const ROLE_PRONOUN_HE: &str = "He / Him / His";
const ROLE_PRONOUN_SHE: &str = "She / Her / Hers";
const DEFAULT_PRONOUN: Pronoun = Pronoun::They;

/// Figures out what pronouns to use for the given user.
///
/// If no pronouns are selected, then [`Pronoun::They`] will be used. If conflicting pronouns are
/// set (i.e if [`Pronoun::He`] and [`Pronoun::She`] are set), then [`Pronoun::They`] will also be
/// used.
pub fn determine_pronouns(ctx: Context<'_>, member: &Member) -> Pronoun {
    let roles = match member.roles(ctx.cache()) {
        Some(roles) => roles,
        None => return DEFAULT_PRONOUN,
    };

    disamiguate_pronouns(roles.iter().map(|role| role.name.as_str()))
}

fn disamiguate_pronouns<'a>(iter: impl Iterator<Item = &'a str>) -> Pronoun {
    iter.filter_map(|name| match name {
        ROLE_PRONOUN_HE => Some(BinaryPronoun::He),
        ROLE_PRONOUN_SHE => Some(BinaryPronoun::She),
        _ => None,
    })
    .fold(None, |acc, pronoun| match acc {
        None => Some(pronoun.into()),
        Some(acc) => {
            if acc == pronoun.into() {
                Some(acc)
            } else {
                Some(DEFAULT_PRONOUN)
            }
        }
    })
    .unwrap_or(DEFAULT_PRONOUN)
}

#[cfg(test)]
mod test {
    use super::*;
    use test_case::test_case;

    const ROLE_PRONOUN_THEY: &str = "They / Them / Theirs";

    #[test_case(&[ROLE_PRONOUN_HE], Pronoun::He; "when with singular he/him")]
    #[test_case(&[ROLE_PRONOUN_SHE], Pronoun::She; "when with singular she/her")]
    #[test_case(&[ROLE_PRONOUN_THEY], Pronoun::They; "when with singular they/them")]
    #[test_case(&[], Pronoun::They; "when with no pronouns")]
    #[test_case(&[ROLE_PRONOUN_HE, ROLE_PRONOUN_SHE], Pronoun::They; "when with conflicting pronouns")]
    #[test_case(&[ROLE_PRONOUN_SHE, ROLE_PRONOUN_HE], Pronoun::They; "when with conflicting pronouns flipped")]
    #[test_case(&[ROLE_PRONOUN_HE, ROLE_PRONOUN_THEY], Pronoun::He; "when with he/they")]
    #[test_case(&[ROLE_PRONOUN_THEY, ROLE_PRONOUN_HE], Pronoun::He; "when with he/they flipped")]
    #[test_case(&[ROLE_PRONOUN_SHE, ROLE_PRONOUN_THEY], Pronoun::She; "when with she/they")]
    #[test_case(&[ROLE_PRONOUN_THEY, ROLE_PRONOUN_SHE], Pronoun::She; "when with she/they flipped")]
    #[test_case(&[ROLE_PRONOUN_SHE, ROLE_PRONOUN_THEY, ROLE_PRONOUN_HE], Pronoun::They; "when with all available pronouns")]
    fn pronoun_tests(pronouns: &[&str], expected_pronoun: Pronoun) {
        let selected_pronoun = disamiguate_pronouns(pronouns.iter().copied());

        assert_eq!(selected_pronoun, expected_pronoun);
    }
}

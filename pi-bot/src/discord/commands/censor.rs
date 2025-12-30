use poise::serenity_prelude::{ChannelId, ChannelType, EditAutoModRule, Trigger, automod::Action};

use crate::discord::{
    Context, Error,
    utils::{CHANNEL_REPORTS, CHANNEL_STAFF},
};

/// Controls Pi-Bot's censor.
#[poise::command(
    slash_command,
    subcommands("add", "remove"),
    default_member_permissions = "MANAGE_MESSAGES",
    required_bot_permissions = "MANAGE_GUILD"
)]
pub async fn censor(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

const PI_BOT_AUTOMOD_RULE: &str = "Pi-Bot - Staff-Submitted Blocked Words";

struct AutomodChannels {
    staff: Option<ChannelId>,
    reports: Option<ChannelId>,
}

/// Staff command. Adds a new entry into the censor.
#[poise::command(slash_command, subcommands("add_word"), guild_only)]
pub async fn add(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Staff command. Removes a word/emoji from the censor list.
#[poise::command(slash_command, subcommands("remove_word"), guild_only)]
pub async fn remove(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(rename = "word", slash_command, guild_only)]
pub async fn add_word(ctx: Context<'_>, word_to_add: String) -> Result<(), Error> {
    let guild = ctx.guild_id().ok_or("Not invoked in guild")?;
    let automod_rules = guild.automod_rules(ctx.http()).await?;

    let channels_for_automod_config = guild.channels(ctx.http()).await?.values().fold(
        AutomodChannels {
            staff: None,
            reports: None,
        },
        |mut acc, channel| {
            if channel.kind == ChannelType::Text {
                if channel.name == CHANNEL_STAFF && acc.staff.is_none() {
                    acc.staff = Some(channel.id);
                } else if channel.name == CHANNEL_REPORTS && acc.reports.is_none() {
                    acc.reports = Some(channel.id);
                }
            }
            acc
        },
    );

    let pibot_automod_rule = match automod_rules
        .iter()
        .find(|rule| rule.name == PI_BOT_AUTOMOD_RULE)
    {
        Some(rule) => rule.clone(),
        None => {
            let mut actions = vec![Action::BlockMessage {
                custom_message:
                    Some("Please refrain from using profanity and other derogatory terms in this server.".to_string()),
            }];
            if let Some(reports_channel) = channels_for_automod_config.reports {
                actions.push(Action::Alert(reports_channel));
            }
            guild
                .create_automod_rule(
                    ctx.http(),
                    EditAutoModRule::new()
                        .name(PI_BOT_AUTOMOD_RULE)
                        .exempt_channels(
                            [
                                channels_for_automod_config.staff,
                                channels_for_automod_config.reports,
                            ]
                            .iter()
                            .filter_map(|&e| e),
                        )
                        .actions(actions)
                        .trigger(Trigger::Keyword {
                            strings: vec![],
                            regex_patterns: vec![],
                            allow_list: vec![],
                        }),
                )
                .await?
        }
    };
    let trigger = match pibot_automod_rule.trigger {
        Trigger::Keyword {
            mut strings,
            regex_patterns,
            allow_list,
        } => {
            strings.push(word_to_add.clone());
            Trigger::Keyword {
                strings,
                regex_patterns,
                allow_list,
            }
        }
        _ => return Err("Automod trigger type is not keyword".into()),
    };
    guild
        .edit_automod_rule(
            ctx.http(),
            pibot_automod_rule.id,
            EditAutoModRule::new().trigger(trigger).enabled(true),
        )
        .await?;
    ctx.reply(format!(
        "Added word `{}` to automod censor ruleset",
        word_to_add
    ))
    .await?;
    Ok(())
}

#[poise::command(rename = "word", slash_command, guild_only)]
pub async fn remove_word(ctx: Context<'_>, word_to_remove: String) -> Result<(), Error> {
    let guild = ctx.guild_id().ok_or("Not invoked in guild")?;
    let automod_rules = guild.automod_rules(ctx.http()).await?;

    let pibot_automod_rule = match automod_rules
        .iter()
        .find(|rule| rule.name == PI_BOT_AUTOMOD_RULE)
    {
        Some(rule) => rule,
        None => {
            ctx.reply("Word list is empty").await?;
            return Ok(());
        }
    };
    let (found_word, trigger) = match pibot_automod_rule.trigger.clone() {
        Trigger::Keyword {
            strings,
            regex_patterns,
            allow_list,
        } => {
            let old_word_list_len = strings.len();
            if old_word_list_len == 0 {
                ctx.reply("Word list is empty").await?;
                return Ok(());
            }
            let new_word_list = strings
                .into_iter()
                .filter(|word| word != &word_to_remove)
                .collect::<Vec<_>>();
            (
                new_word_list.len() != old_word_list_len,
                Trigger::Keyword {
                    strings: new_word_list,
                    regex_patterns,
                    allow_list,
                },
            )
        }
        _ => return Err("Automod trigger type is not keyword".into()),
    };

    if found_word {
        guild
            .edit_automod_rule(
                ctx.http(),
                pibot_automod_rule.id,
                EditAutoModRule::new().trigger(trigger).enabled(true),
            )
            .await?;
        ctx.reply(format!(
            "Removed word `{}` from automod censor ruleset",
            word_to_remove
        ))
        .await?;
    } else {
        ctx.reply(format!(
            "Word `{}` is not within automod censor ruleset",
            word_to_remove
        ))
        .await?;
    }
    Ok(())
}

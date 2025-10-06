use poise::{
    CreateReply,
    serenity_prelude::{
        ChannelType, Colour, CreateEmbed, DefaultMessageNotificationLevel, ExplicitContentFilter,
        FormattedTimestamp, Mentionable, MfaLevel, PremiumTier, VerificationLevel,
    },
};

use crate::{
    discord::{Context, Error},
    version::VERSION,
};
use std::{collections::HashMap, ops::Deref};

#[derive(Debug, poise::ChoiceParameter)]
enum SciolyOrgService {
    Forums,
    Wiki,
    #[name = "Test Exchange"]
    TestExchange,
    Gallery,
    #[name = "OBB"]
    Obb,
    Tournaments,
}

impl SciolyOrgService {
    pub fn get_link(&self) -> String {
        let path = match self {
            Self::Forums => "forums",
            Self::Wiki => "wiki",
            Self::TestExchange => "tests",
            Self::Gallery => "gallery",
            Self::Obb => "obb",
            Self::Tournaments => "tournaments",
        };
        format!("https://scioly.org/{}", path)
    }
}

#[derive(Debug)]
struct GuildLimit {
    emoji: u64,
    _stickers: u64,
    bitrate: u64,
    filesize: u64,
}

/// Returns information about the bot and server.
#[poise::command(slash_command, member_cooldown = 20, member_cooldown_burst = 2)]
pub async fn about(ctx: Context<'_>) -> Result<(), Error> {
    let repo = "https://github.com/scioly/pi-bot";
    let wiki_link = "https://scioly.org/wiki/User:Pi-Bot";
    let forums_link = "https://scioly.org/forums/memberlist.php?mode=viewprofile&u=62443";
    let avatar_url = ctx.cache().current_user().avatar_url();

    let mut embed = CreateEmbed::default()
        .title(format!("**Pi-Bot {}**", VERSION))
        .color(Colour::from_rgb(0xF8, 0x6D, 0x5F))
        .description(
            "Hey there! I'm Pi-Bot, and I help to manage the Scioly.org forums, \
            wiki, and chat. You'll often see me around this Discord server to help users get roles \
            and information about Science Olympiad.\n\
            \n\
            I'm developed by the community. If you'd like to find more about development, you can \
            find more by visiting the links below.",
        )
        .field("Code Repository", repo, false)
        .field("Wiki Page", wiki_link, false)
        .field("Forums Page", forums_link, false);

    if let Some(avatar_url) = avatar_url {
        embed = embed.thumbnail(avatar_url);
    }

    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Returns the Discord server invite.
#[poise::command(slash_command, member_cooldown = 60, member_cooldown_burst = 5)]
pub async fn invite(ctx: Context<'_>) -> Result<(), Error> {
    ctx.reply("https://discord.gg/scioly").await?;
    Ok(())
}

/// Returns a link to the Scioly.org forums.
#[poise::command(slash_command, member_cooldown = 60, member_cooldown_burst = 5)]
pub async fn link(ctx: Context<'_>, destination: SciolyOrgService) -> Result<(), Error> {
    ctx.reply(format!("<{}>", destination.get_link())).await?;
    Ok(())
}

/// Returns a random number, inclusively.
#[poise::command(slash_command, member_cooldown = 60, member_cooldown_burst = 5)]
pub async fn random(
    ctx: Context<'_>,
    #[description = "The minimum number to choose from. Defaults to 0."] minimum: Option<u64>,
    #[description = "The maximum number to choose from. Defaults to 10."] maximum: Option<u64>,
) -> Result<(), Error> {
    let mut minimum = minimum.unwrap_or(0);
    let mut maximum = maximum.unwrap_or(10);
    if maximum < minimum {
        std::mem::swap(&mut maximum, &mut minimum);
    }
    let num = rand::random_range(minimum..=maximum);
    ctx.reply(format!(
        "Random number between `{}` and `{}`: `{}`",
        minimum, maximum, num
    ))
    .await?;
    Ok(())
}

/// Information about gaining the @Coach role.
#[poise::command(slash_command, member_cooldown = 60, member_cooldown_burst = 5)]
pub async fn coach(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(
        CreateReply::default()
            .content(
                "If you would like to apply for the `Coach` role, please fill out the form here: \
            <https://forms.gle/UBKpWgqCr9Hjw9sa6>.",
            )
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

/// Information about the current server.
#[poise::command(
    slash_command,
    guild_only,
    member_cooldown = 60,
    member_cooldown_burst = 5
)]
pub async fn info(ctx: Context<'_>) -> Result<(), Error> {
    const CHANNEL_LIMIT: u32 = 500;
    const ROLE_LIMIT: u32 = 250;
    let premium_guild_limits: HashMap<Option<PremiumTier>, GuildLimit> = HashMap::from([
        (
            None,
            GuildLimit {
                emoji: 50,
                _stickers: 5,
                bitrate: 96_000,
                filesize: 10485760,
            },
        ),
        (
            Some(PremiumTier::Tier0),
            GuildLimit {
                emoji: 50,
                _stickers: 5,
                bitrate: 96_000,
                filesize: 10485760,
            },
        ),
        (
            Some(PremiumTier::Tier1),
            GuildLimit {
                emoji: 100,
                _stickers: 15,
                bitrate: 128_000,
                filesize: 10485760,
            },
        ),
        (
            Some(PremiumTier::Tier2),
            GuildLimit {
                emoji: 150,
                _stickers: 30,
                bitrate: 256_000,
                filesize: 52428800,
            },
        ),
        (
            Some(PremiumTier::Tier3),
            GuildLimit {
                emoji: 250,
                _stickers: 60,
                bitrate: 384_000,
                filesize: 104857600,
            },
        ),
    ]);
    let server = ctx.guild().unwrap().deref().clone();
    let name = server.name.clone();
    let owner = server.owner_id.to_user(ctx.http()).await?;
    let creation_date = FormattedTimestamp::new(
        server.id.created_at(),
        Some(poise::serenity_prelude::FormattedTimestampStyle::RelativeTime),
    );
    let emoji_count = server.emojis.len();
    let icon = server.icon_url();
    let animated_icon = server.icon.is_some_and(|hash| hash.is_animated());
    let iden = server.id;
    let banner = server.banner_url();
    let desc = server.description.clone();
    let mfa_level = server.mfa_level;
    let verification_level = server.verification_level;
    let content_filter = server.explicit_content_filter;
    let default_notifs = server.default_message_notifications;
    let features = server.features.clone();
    let splash = server.splash_url();
    let premium_level = server.premium_tier;
    let premium_level_str = match premium_level {
        PremiumTier::Tier0 => "Tier 0".into(),
        PremiumTier::Tier1 => "Tier 1".into(),
        PremiumTier::Tier2 => "Tier 2".into(),
        PremiumTier::Tier3 => "Tier 3".into(),
        PremiumTier::Unknown(level) => format!("Unknown Tier ({})", level),
        _ => "Unknown".into(),
    };
    let boosts = server.premium_subscription_count;
    let channel_count = server.channels.len();
    let channel_counts = server.channels(ctx.http()).await?.iter().fold(
        HashMap::from([
            (ChannelType::Text, 0_u32),
            (ChannelType::Voice, 0_u32),
            (ChannelType::Category, 0_u32),
        ]),
        |mut acc, (_, channel)| {
            acc.entry(channel.kind).and_modify(|count| *count += 1);
            acc
        },
    );
    let text_channel_count = channel_counts.get(&ChannelType::Text).unwrap();
    let voice_channel_count = channel_counts.get(&ChannelType::Voice).unwrap();
    let category_count = channel_counts.get(&ChannelType::Category).unwrap();
    let system_channel = server
        .system_channel_id
        .map(async |id| id.to_channel(ctx.http()).await);
    let system_channel_mention = if let Some(future) = system_channel {
        let channel = future.await?;
        channel.mention().to_string()
    } else {
        "None".into()
    };
    let rules_channel = server
        .rules_channel_id
        .map(async |id| id.to_channel(ctx.http()).await);
    let rules_channel_mention = if let Some(future) = rules_channel {
        let channel = future.await?;
        channel.mention().to_string()
    } else {
        "None".into()
    };
    let public_updates_channel = server
        .public_updates_channel_id
        .map(async |id| id.to_channel(ctx.http()).await);
    let public_updates_channel_mention = if let Some(future) = public_updates_channel {
        let channel = future.await?;
        channel.mention().to_string()
    } else {
        "None".into()
    };
    let guild_limits = premium_guild_limits.get(&Some(premium_level)).unwrap();
    let emoji_limit = {
        let more_emoji = if features.iter().any(|feature| feature == "MORE_EMOJI") {
            200
        } else {
            50
        };
        guild_limits.emoji.max(more_emoji)
    };
    let bitrate_limit = {
        let vip_guild = if features.iter().any(|feature| feature == "VIP_REGIONS") {
            premium_guild_limits
                .get(&Some(PremiumTier::Tier1))
                .unwrap()
                .bitrate
        } else {
            96_000
        };
        guild_limits.bitrate.max(vip_guild)
    };

    let filesize_limit = (guild_limits.filesize as f64 / 1_000_f64).round() / 1_000_f64;
    let boosters = server
        .members
        .iter()
        .filter_map(|(_, member)| member.premium_since.map(|_| member.mention().to_string()))
        .collect::<Box<[_]>>()
        .join(", ");

    let role_count = server.roles.len();
    let member_count = server.members.len();
    let max_members = server.max_members;
    let discovery_splash_url = server.discovery_splash.map(|splash| {
        let format = if splash.to_string().starts_with("a_") {
            "gif"
        } else {
            "png"
        };

        format!(
            "https://cdn.discordapp.com/discovery-splashes/{}/{}.{}?size=1024",
            server.id.get(),
            splash,
            format
        )
    });
    let member_percentage = if let Some(max_members) = max_members {
        ((member_count as f64 / max_members as f64) * 100_000_f64).round() / 1_000_f64
    } else {
        0_f64
    };
    let emoji_percentage =
        ((emoji_count as f64 / emoji_limit as f64) * 100_000_f64).round() / 1_000_f64;
    let channel_percentage =
        (channel_count as f64 / CHANNEL_LIMIT as f64 * 100_000_f64).round() / 1_000_f64;
    let role_percentage = (role_count as f64 / ROLE_LIMIT as f64 * 100_000_f64).round() / 1_000_f64;

    let mut fields = vec![
        (
            "Basic Information",
            format!(
                "**Creation Date:** {}\n\
                         **ID:** {}\n\
                         **Animated Icon:** {}\n\
                         **Banner URL:** {}\n\
                         **Splash URL:** {}\n\
                         **Discovery Splash URL:** {}",
                creation_date,
                iden,
                animated_icon,
                banner.unwrap_or("None".into()),
                splash.unwrap_or("None".into()),
                discovery_splash_url.unwrap_or("None".into())
            ),
            false,
        ),
        (
            "Nitro Information",
            format!(
                "**Nitro Level:** {} ({} individual boosts)\n\
                **Boosters:** {}",
                premium_level_str,
                boosts.map_or("Unknown".into(), |count| format!("{}", count)),
                boosters
            ),
            false,
        ),
    ];
    if let Some(channel) = ctx.guild_channel().await
        && let Some(parent_channel_id) = channel.parent_id
    {
        let parent_channel = parent_channel_id
            .to_channel(ctx.http())
            .await?
            .guild()
            .unwrap();
        const CATEGORY_STAFF: &str = "staff";
        if matches!(parent_channel.kind, ChannelType::Category)
            && parent_channel.name() == CATEGORY_STAFF
        {
            fields.append(&mut vec![
                (
                    "Staff Information",
                    format!(
                        "**Owner:** {}\n\
                            **MFA Level:** {}\n\
                            **Verification Level:** {}\n\
                            **Content Filter:** {}\n\
                            **Default Notifications:** {}\n\
                            **Features:** {:?}\n\
                            **Bitrate Limit:** {}\n\
                            **Filesize Limit:** {} MB",
                        owner,
                        match mfa_level {
                            MfaLevel::None => "None".into(),
                            MfaLevel::Elevated => "Require 2FA".into(),
                            MfaLevel::Unknown(level) => format!("Unknown ({})", level),
                            _ => "Unknown".into(),
                        },
                        match verification_level {
                            VerificationLevel::None => "None".into(),
                            VerificationLevel::Low => "Low".into(),
                            VerificationLevel::Medium => "Medium".into(),
                            VerificationLevel::High => "High".into(),
                            VerificationLevel::Higher => "Higher".into(),
                            VerificationLevel::Unknown(level) => format!("Unknown ({})", level),
                            _ => "Unknown".into(),
                        },
                        match content_filter {
                            ExplicitContentFilter::None => "None".into(),
                            ExplicitContentFilter::WithoutRole => "Members without role".into(),
                            ExplicitContentFilter::All => "All members".into(),
                            ExplicitContentFilter::Unknown(level) => format!("Unknown ({})", level),
                            _ => "Unknown".into(),
                        },
                        match default_notifs {
                            DefaultMessageNotificationLevel::All => "All".into(),
                            DefaultMessageNotificationLevel::Mentions => "Mentions only".into(),
                            DefaultMessageNotificationLevel::Unknown(level) =>
                                format!("Unknown ({})", level),
                            _ => "Unknown".into(),
                        },
                        features,
                        bitrate_limit,
                        filesize_limit
                    ),
                    false,
                ),
                (
                    "Channels",
                    format!(
                        "**Public Updates Channel:** {}\n\
                        **System Channel:** {}\n\
                        **Rules Channel:** {}\n\
                        **Text Channel Count:** {}\n\
                        **Voice Channel Count:** {}\n\
                        **Category Count:** {}\n",
                        public_updates_channel_mention,
                        system_channel_mention,
                        rules_channel_mention,
                        text_channel_count,
                        voice_channel_count,
                        category_count
                    ),
                    false,
                ),
                (
                    "Limits",
                    format!(
                        "**Channels:** *{}%* ({}/{} channels)\n\
                        **Members:** *{}%* ({}/{} members)\n\
                        **Emoji:** *{}%* ({}/{} emojis)\n\
                        **Roles:** *{}%* ({}/{} roles)",
                        channel_percentage,
                        channel_count,
                        CHANNEL_LIMIT,
                        member_percentage,
                        member_count,
                        max_members.map_or("Unknown".into(), |count| format!("{}", count)),
                        emoji_percentage,
                        emoji_count,
                        emoji_limit,
                        role_percentage,
                        role_count,
                        ROLE_LIMIT,
                    ),
                    false,
                ),
            ]);
        }
    }

    let mut embed = CreateEmbed::default()
        .title(format!("Information for `{}`", name))
        .description(format!("**Description:** {}", desc.unwrap_or("".into())))
        .fields(fields);
    if let Some(icon) = icon {
        embed = embed.thumbnail(icon);
    }
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

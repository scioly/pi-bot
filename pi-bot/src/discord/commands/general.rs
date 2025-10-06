use poise::{
    CreateReply,
    serenity_prelude::{Colour, CreateEmbed},
};

use crate::{
    discord::{Context, Error},
    version::VERSION,
};

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

use std::time::Duration;

use num_bigint::BigUint;
use poise::{
    CreateReply,
    serenity_prelude::{CreateEmbed, Member, Mentionable},
};
use rand::{Rng, SeedableRng, rngs::StdRng};
use tokio::sync::Mutex;

use crate::discord::{Context, Error};

static FISH_COUNT: Mutex<BigUint> = Mutex::const_new(BigUint::ZERO);

// # of bits to store 10^1_000_000 (i.e. 1_000_000 / log_10(2))
const MAX_BITS_ALLOWED: u64 = 3_321_929;

#[derive(Debug, poise::ChoiceParameter)]
enum SnackOption {
    #[name = "chocolate bar"]
    ChocolateBar,
    #[name = "cookie"]
    Cookie,
    #[name = "ice cream"]
    IceCream,
    #[name = "pizza"]
    Pizza,
    #[name = "boba"]
    Boba,
    #[name = "a slice of cake"]
    SliceOfCake,
    #[name = "chips and salsa"]
    ChipsAndSalsa,
    #[name = "brownie"]
    Brownie,
    #[name = "cotton candy"]
    CottonCandy,
}

/// Gives a fish to bear.
#[poise::command(slash_command, member_cooldown = 10, member_cooldown_burst = 5)]
pub async fn fish(ctx: Context<'_>) -> Result<(), Error> {
    let mut rng = StdRng::from_os_rng();
    let mut fish = FISH_COUNT.lock().await;

    if fish.bits() >= MAX_BITS_ALLOWED {
        *fish = fish.sqrt();
        ctx.reply(
            "Woah! Bear's fish is a little too high, so it unfortunately has to be square rooted.",
        )
        .await?;
        return Ok(());
    }

    let n = rng.random_range(0_u32..100);

    match n {
        100.. => unreachable!(), // FIXME: Err(anyhow!("Unreachable")),
        90.. => {
            *fish += 10_u32;
            ctx.reply(format!(
                "Wow, you gave bear a super fish! Added 10 fish! Bear now has {} fish!",
                fish
            ))
            .await?;
            Ok(())
        }
        10.. => {
            *fish += 1_u32;
            ctx.reply(format!(
                "You feed bear one fish. Bear now has {} fish!",
                fish
            ))
            .await?;
            Ok(())
        }
        2.. => {
            ctx.reply(format!(
                "You can't find any fish... and thus can't feed bear. Bear still has {} fish.",
                fish,
            ))
            .await?;
            Ok(())
        }
        0.. => {
            *fish = fish.sqrt();
            ctx.reply(
                format!(
                    ":sob:\n:sob:\n:sob:\nAww, bear's fish was accidentally square root'ed. Bear now has {} fish. \n:sob:\n:sob:\n:sob: ", fish)
            ).await?;
            Ok(())
        }
    }
}

#[poise::command(slash_command, member_cooldown = 10, member_cooldown_burst = 5)]
pub async fn stealfish(ctx: Context<'_>) -> Result<(), Error> {
    let mut rng = StdRng::from_os_rng();
    let mut fish = FISH_COUNT.lock().await;

    let n = rng.random_range(0_u32..1000);

    match n {
        1000.. => unreachable!(), // FIXME: Err(anyhow!("Unreachable")),
        750.. => {
            let ratio = (n - 500) as f64 / 1000_f64;
            *fish = fish.clone() * BigUint::from(1500_u32 - n) / BigUint::from(1000_u32);
            let per = (ratio * 100_f64) as u32;
            ctx.reply(format!("You stole {}% of bear's fish!", per))
                .await?;
            Ok(())
        }
        416.. => {
            *fish = fish.clone() * BigUint::from(99_u32) / BigUint::from(100_u32);
            ctx.reply("You stole just 1% of bear's fish!").await?;
            Ok(())
        }
        250.. => {
            *fish = fish.clone() * BigUint::from(750 + n) / BigUint::from(1000_u32);
            let per = ((n + 750 - 1000) as f64 / 10_f64) as u32;
            ctx.reply(format!(
                "Uhh... something went wrong! You gave bear another {}% of his fish!",
                per,
            ))
            .await?;
            Ok(())
        }
        0.. => {
            ctx.reply("Hmm, nothing happened. *crickets*").await?;
            Ok(())
        }
    }
}

/// Trout slaps yourself or another user!
#[poise::command(slash_command, member_cooldown = 60, member_cooldown_burst = 5)]
pub async fn trout(
    ctx: Context<'_>,
    #[description = "The member to trout slap! If not given, Pi-Bot will trout slap you!"]
    member: Option<Member>,
) -> Result<(), Error> {
    let member_str = if let Some(member) = member {
        member.user.mention().to_string()
    } else {
        "themself".to_string()
    };

    let embed = CreateEmbed::default()
        .description(format!(
            "{} slaps {} with a giant trout!",
            ctx.author().mention(),
            member_str
        ))
        .image("https://media4.giphy.com/media/v1.Y2lkPTc5MGI3NjExNmZtdDlkZHptaDR4YmV3dDM2Y21oYTMxbXIydTM5bmVjY2NiMGxwOSZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/rgjwOLuv0azHW/giphy.gif");
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Gives a treat to yourself or another user!
#[poise::command(slash_command, member_cooldown = 60, member_cooldown_burst = 5)]
pub async fn treat(
    ctx: Context<'_>,
    #[description = "The treat you want to give."] snack: SnackOption,
    #[description = "The member to give the treat to! Defaults to yourself!"] member: Option<
        Member,
    >,
) -> Result<(), Error> {
    let (treat_phrase, images): (_, &[_]) = match snack {
        SnackOption::ChocolateBar => (
            "a chocolate bar",
            &[
                "https://media2.giphy.com/media/v1.Y2lkPTc5MGI3NjExemg5amo1ZzQwZXRxd2F3MWprazZtb2l0Mm0zYm5idnB6cnp0d253OCZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/3oxOCiTugN67UB9bRS/giphy.webp",
                "https://media.giphy.com/media/Wrscj8qsDogR4QHx2j/giphy.gif",
                "https://media.giphy.com/media/gIqguY2jmB31LZqWip/giphy.gif",
                "https://media.giphy.com/media/xUA7aUV3sYqsCkRLa0/giphy.gif",
                "https://media.giphy.com/media/gGwL4lMFOdSsGQlJEG/giphy.gif",
            ],
        ),
        SnackOption::IceCream => (
            "ice cream",
            &[
                "https://media4.giphy.com/media/v1.Y2lkPTc5MGI3NjExbTN6eDBtc3h0cGV1Yjc3cXRyNDZkYW5yeHExeWd2NzVqdDdtaXo0ciZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/aagX4Bl8Fo21G/giphy.webp",
                "https://media.giphy.com/media/PB5E8c20NXslUuIxna/giphy.gif",
                "https://media.giphy.com/media/CqS6nhPTCu6e5v2R03/giphy.gif",
                "https://media.giphy.com/media/GB91uLrgyuul2/giphy.gif",
                "https://media.giphy.com/media/uUs14eCA2SBgs/giphy-downsized-large.gif",
                "https://media.giphy.com/media/26uf7yJapo82e48yA/giphy.gif",
            ],
        ),
        SnackOption::Cookie => (
            "a cookie",
            &[
                "https://media0.giphy.com/avatars/pusheen/Lfx3DRTbYeG6/80h.gif",
                "https://media.giphy.com/media/59Ve1fnBdol8c/giphy.gif",
                "https://media.giphy.com/media/JIPEUnwfxjtT0OapJb/giphy.gif",
                "https://media.giphy.com/media/26FeXTOe2R9kfpObC/giphy.gif",
                "https://media.giphy.com/media/EKUvB9uFnm2Xe/giphy.gif",
                "https://media.giphy.com/media/38a2gPetE4RuE/giphy-downsized-large.gif",
                "https://media.giphy.com/media/c7maSqDI7j2ww/giphy.gif",
            ],
        ),
        SnackOption::Pizza => (
            "pizza",
            &[
                "https://media.giphy.com/media/3osxYoufeOGOA7xiX6/giphy.gif",
                "https://media.giphy.com/media/1108D2tVaUN3eo/giphy.gif",
                "https://media.giphy.com/media/QR7ci2sbhrkzxAuMHH/giphy.gif",
                "https://media.giphy.com/media/hmzAcor7gBsbK/giphy-downsized-large.gif",
                "https://media.giphy.com/media/aCKMaeduKfFXG/giphy.gif",
            ],
        ),
        SnackOption::Boba => (
            "boba",
            &[
                "https://media.giphy.com/media/7SZzZO5EG1S6QLJeUL/giphy.gif",
                "https://media.giphy.com/media/r6P5BC5b4SS2Y/giphy.gif",
                "https://media.giphy.com/media/cRLPmyXQhtRXnRXfDX/giphy.gif",
                "https://media.giphy.com/media/h8CD39vtPVoMEoqZZ3/giphy.gif",
                "https://media.giphy.com/media/Y4VNo2dIdW8bpDgRXt/giphy.gif",
            ],
        ),
        SnackOption::SliceOfCake => (
            "a slice of cake",
            &[
                "https://media.giphy.com/media/He4wudo59enf2/giphy.gif",
                "https://media.giphy.com/media/l0Iy4ppWvwQ4SXPxK/giphy.gif",
                "https://media.giphy.com/media/zBU43ZvUVj37a/giphy.gif",
                "https://media.giphy.com/media/wPamPmbGkWkQE/giphy.gif",
                "https://media.giphy.com/media/JMfzwxEIbd6zC/giphy.gif",
            ],
        ),
        SnackOption::ChipsAndSalsa => (
            "chips and salsa, I suppose",
            &[
                "https://media.giphy.com/media/xThuWwvZWJ4NOB6j6w/giphy.gif",
                "https://media.giphy.com/media/wZOF08rE9knDTYsY4G/giphy.gif",
                "https://media.giphy.com/media/1O3nlwRXcOJYLv1Neh/giphy.gif",
                "https://media.giphy.com/media/YrN8O2eGl2f5GucpEf/giphy.gif",
            ],
        ),
        SnackOption::Brownie => (
            "a brownie",
            &[
                "https://media.giphy.com/media/BkWHoSRB6gR2M/giphy.gif",
                "https://media.giphy.com/media/abOlz9ygIm9Es/giphy.gif",
                "https://media.giphy.com/media/l0MYEU0YyoTEpTDby/giphy-downsized-large.gif",
                "https://media.giphy.com/media/VdQ8b54TJZ9kXClaSw/giphy.gif",
                "https://media.giphy.com/media/ziuCU2H0DdtGoZdJu3/giphy.gif",
            ],
        ),
        SnackOption::CottonCandy => (
            "cotton candy",
            &[
                "https://media.giphy.com/media/1X7A3s673cLWovCQCE/giphy.gif",
                "https://media.giphy.com/media/dXKH2jCT9tINyVWlUp/giphy.gif",
                "https://media.giphy.com/media/V83Khg0lCKyOc/giphy.gif",
                "https://media.giphy.com/media/ZcVI712Fcol3EeLltH/giphy-downsized-large.gif",
            ],
        ),
    };

    let member_str = if let Some(member) = member {
        member.mention().to_string()
    } else {
        "themself".to_string()
    };

    let idx = rand::random_range(0..images.len());
    let embed = CreateEmbed::default()
        .description(format!(
            "{} gives {} {}!",
            ctx.author().mention(),
            member_str,
            treat_phrase
        ))
        .image(images[idx]);
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Rolls the magic 8 ball...
#[poise::command(slash_command, member_cooldown = 60, member_cooldown_burst = 5)]
pub async fn magic8ball(ctx: Context<'_>) -> Result<(), Error> {
    let switching_ball_message = "Swishing the magic 8 ball...";
    let reply = ctx.reply(switching_ball_message).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let switching_ball_message = switching_ball_message
        .strip_suffix(|_| true)
        .unwrap_or(switching_ball_message);
    reply
        .edit(ctx, CreateReply::default().content(switching_ball_message))
        .await?;

    tokio::time::sleep(Duration::from_secs(1)).await;

    let switching_ball_message = switching_ball_message
        .strip_suffix(|_| true)
        .unwrap_or(switching_ball_message);
    reply
        .edit(ctx, CreateReply::default().content(switching_ball_message))
        .await?;

    let sayings = [
        "Yes.",
        "Ask again later.",
        "Not looking good.",
        "Cannot predict now.",
        "It is certain.",
        "Try again.",
        "Without a doubt.",
        "Don't rely on it.",
        "Outlook good.",
        "My reply is no.",
        "Don't count on it.",
        "Yes - definitely.",
        "Signs point to yes.",
        "I believe so.",
        "Nope.",
        "Concentrate and ask later.",
        "Try asking again.",
        "For sure not.",
        "Definitely no.",
    ];
    let idx = rand::random_range(0..sayings.len());
    let response = sayings[idx];
    reply
        .edit(
            ctx,
            CreateReply::default().content(format!("**{}**", response)),
        )
        .await?;
    Ok(())
}

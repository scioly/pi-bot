from __future__ import annotations

import logging
from typing import TYPE_CHECKING

import discord
from beanie.odm.operators.update.general import Set
from discord import AllowedMentions, Member, RawMemberRemoveEvent, app_commands
from discord.ext import commands
from motor.core import ClientSession

from env import env
from src.discord.globals import (
    ROLE_EM,
    ROLE_LH,
    ROLE_MUTED,
    ROLE_QUARANTINE,
    ROLE_SELFMUTE,
    ROLE_STAFF,
    ROLE_VIP,
)
from src.mongo.models import UserRoles

if TYPE_CHECKING:
    from bot import PiBot

NON_PUBLIC_ROLES = [
    # Note that any role above `Bot` or `Pi-Bot` will not be assignable by the bot. This essentially
    # means any Staff role is not assignable and existing staff must assign the user those roles manually.
    ROLE_LH,
    ROLE_EM,
    ROLE_MUTED,
    ROLE_SELFMUTE,
    ROLE_QUARANTINE,
]


class RoleRestore(commands.Cog):
    """
    Cog responsible for maintaining members roles when they leave and rejoin. This is in particular
    to *awarded* roles and not roles that are publicly assignable.
    """

    def __init__(self, bot: PiBot):
        self.bot = bot

    rolerestore_group = app_commands.Group(
        name="rolerestore",
        description="Commands pertaining to RoleRestore.",
        guild_ids=env.slash_command_guilds,
        default_permissions=discord.Permissions(manage_roles=True),
    )

    @rolerestore_group.command(
        name="sync",
        description="Staff command. Syncs Pi-Bot's backend with the current state of a user's non-public roles.",
    )
    @app_commands.checks.has_any_role(ROLE_STAFF, ROLE_VIP)
    @app_commands.describe(
        member="Sync a particular user rather than the entire server.",
    )
    async def sync(self, interaction: discord.Interaction, member: Member):
        logging.info("Manually syncing roles for user `%s`", member.name)
        await interaction.response.send_message(
            f"Syncing roles for {member.mention}",
            allowed_mentions=AllowedMentions.none(),
        )
        roles = await sync_roles(member, None)

        roles_markdown = [f"`{discord.utils.escape_markdown(role)}`" for role in roles]
        roles_str = ", ".join(roles_markdown)
        await interaction.edit_original_response(
            content=f"The following roles were synced: {roles_str}",
            allowed_mentions=AllowedMentions.none(),
        )
        return

    @commands.Cog.listener()
    async def on_raw_member_remove(self, payload: RawMemberRemoveEvent):
        if isinstance(payload.user, discord.User):
            # figure out if we can find member here
            # the payload type .user can be User | Member, but this should only be called when the member leaves a Guild??
            return

        logging.info(
            "Syncing roles for `%s` (triggered by on_raw_member_remove)",
            payload.user.name,
        )
        await sync_roles(payload.user, None)

    @commands.Cog.listener()
    async def on_member_join(self, member: Member):
        logging.info("Restoring roles for user `%s`", member.name)
        user_roles = await UserRoles.find_one(
            UserRoles.user_id == member.id,
            UserRoles.guild_id == member.guild.id,
        )

        if user_roles:
            roles_to_add = []
            for role_name in user_roles.roles:
                role = discord.utils.get(member.guild.roles, name=role_name)
                roles_to_add.append(role)

            await member.add_roles(
                *roles_to_add,
                reason="Existing user rejoined. Restoring non-public roles.",
            )
            logging.info("Finish restoring roles for `%s`", member.name)
        else:
            logging.info(
                "RoleRestore could not find existing entry for `%s` for restoration",
                member.name,
            )

    @commands.Cog.listener()
    async def on_member_update(self, _: Member, after: Member):
        logging.info("Updating roles for user `%s`", after.name)
        # TODO: add logic to prevent unnecessary syncs
        await sync_roles(after, None)


async def sync_roles(member: Member, session: ClientSession | None):
    roles = [role.name for role in member.roles if role.name in NON_PUBLIC_ROLES]

    await UserRoles.find_one(
        UserRoles.user_id == member.id,
        UserRoles.guild_id == member.guild.id,
        session=session,
    ).upsert(
        Set({UserRoles.roles: roles}),
        on_insert=UserRoles(user_id=member.id, guild_id=member.guild.id, roles=roles),
        session=session,
    )

    logging.info(
        "%d roles were saved for `%s` (id: %d)",
        len(roles),
        member.name,
        member.id,
    )

    return roles


async def setup(bot: PiBot):
    await bot.add_cog(RoleRestore(bot))

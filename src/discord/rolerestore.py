from __future__ import annotations

import logging
from typing import TYPE_CHECKING

import discord
from beanie.odm.operators.update.general import Set
from discord import Member, RawMemberRemoveEvent
from discord.ext import commands

from src.discord.globals import (
    ROLE_AD,
    ROLE_BT,
    ROLE_EM,
    ROLE_GM,
    ROLE_LH,
    ROLE_MUTED,
    ROLE_QUARANTINE,
    ROLE_SELFMUTE,
    ROLE_STAFF,
    ROLE_VIP,
    ROLE_WM,
)
from src.mongo.models import UserRoles

if TYPE_CHECKING:
    from bot import PiBot

NON_PUBLIC_ROLES = [
    ROLE_WM,
    ROLE_GM,
    ROLE_AD,
    ROLE_VIP,
    ROLE_STAFF,
    ROLE_BT,
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

    async def on_raw_member_remove(self, payload: RawMemberRemoveEvent):
        if isinstance(payload.user, discord.User):
            # figure out if we can find member here
            # the payload type .user can be User | Member, but this should only be called when the member leaves a Guild??
            return

        roles_to_save = [
            role.name for role in payload.user.roles if role.name in NON_PUBLIC_ROLES
        ]

        await UserRoles.find_one(
            UserRoles.user_id == payload.user.id,
            UserRoles.guild_id == payload.guild_id,
        ).upsert(
            Set({UserRoles.roles: roles_to_save}),
            on_insert=UserRoles(
                user_id=payload.user.id,
                guild_id=payload.guild_id,
                roles=roles_to_save,
            ),
        )

        logging.info(
            "{} roles were saved for `{}` (id: {})".format(
                len(roles_to_save),
                payload.user.name,
                payload.user.id,
            ),
        )

    async def on_member_join(self, member: Member):
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


async def setup(bot: PiBot):
    await bot.add_cog(RoleRestore(bot))

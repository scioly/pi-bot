from __future__ import annotations

import re
from typing import TYPE_CHECKING, Literal

import discord
from discord import DiscordException, Role, app_commands
from discord.ext import commands

import commandchecks
import src.discord.globals
from env import env
from src.discord.globals import (
    EMOJI_LOADING,
    ROLE_STAFF,
    ROLE_VIP,
)
from src.mongo.models import Event

if TYPE_CHECKING:
    from bot import PiBot

EVENT_ROLE_HEX_COLOR = 0x82A3D3

EnableDisable = Literal["enable", "disable"]


class StaffEvents(commands.Cog):
    def __init__(self, bot: PiBot):
        self.bot = bot

    event_commands_group = app_commands.Group(
        name="event",
        description="Updates the bot's list of events.",
        guild_ids=env.slash_command_guilds,
        default_permissions=discord.Permissions(manage_roles=True),
    )

    event_batch_commands_group = app_commands.Group(
        name="batch",
        description="Updates the bot's list of events.",
        guild_ids=env.slash_command_guilds,
        default_permissions=discord.Permissions(manage_roles=True),
        parent=event_commands_group,
    )

    def fetch_role(self, name: str) -> Role | None:
        server = self.bot.get_guild(env.server_id)
        potential_role = discord.utils.get(server.roles, name=name)

        return potential_role

    async def create_event_role(
        self,
        interaction: discord.Interaction,
        event_name: str,
    ) -> Role:
        """
        NOTE: this function does not check for existing names of roles.
        """
        reason_stmt = f"Created by using {interaction.user!s}"
        if interaction.command:
            reason_stmt += f" {interaction.command.name}"
        reason_stmt += " with Pi-Bot."

        # Create role on server
        server = self.bot.get_guild(env.server_id)
        assert isinstance(server, discord.Guild)
        return await server.create_role(
            name=event_name,
            color=discord.Color(EVENT_ROLE_HEX_COLOR),
            reason=reason_stmt,
        )

    @event_commands_group.command(
        name="add",
        description="Staff command. Adds a new event.",
    )
    @app_commands.checks.has_any_role(ROLE_STAFF, ROLE_VIP)
    @app_commands.describe(
        event_name="The name of the new event.",
        event_aliases="The aliases for the new event. Format as 'alias1, alias2'.",
    )
    async def event_add(
        self,
        interaction: discord.Interaction,
        event_name: str,
        event_aliases: str | None = None,
    ):
        # Check for staff permissions
        commandchecks.is_staff_from_ctx(interaction)

        # Send user notice that process has begun
        await interaction.response.send_message(
            f"{EMOJI_LOADING} Attempting to add `{event_name}` as a new event...",
        )

        # Check to see if event has already been added.
        if event_name in [e.name for e in src.discord.globals.EVENT_INFO]:
            return await interaction.edit_original_response(
                content=f"The `{event_name}` event has already been added.",
            )

        if self.fetch_role(event_name):
            return await interaction.edit_original_response(
                content="A role with the name `{}` already exists. The event cannot be added until "
                "that role has been manually deleted.".format(event_name),
            )
        # Construct dictionary to represent event; will be stored in database
        # and local storage
        aliases_array = []
        if event_aliases:
            aliases_array = re.findall(r"\w+", event_aliases)
        new_dict = Event(name=event_name, aliases=aliases_array, emoji=None)

        new_role = await self.create_event_role(interaction, event_name)

        try:
            # Add dict into events container
            await new_dict.insert()
            src.discord.globals.EVENT_INFO.append(new_dict)
        except Exception as e:
            # If db fails, cleanup role
            await new_role.delete()
            raise e

        # Notify user of process completion
        await interaction.edit_original_response(
            content=f"The `{event_name}` event was added.",
        )

    async def enable_role(
        self,
        interaction: discord.Interaction,
        event_name: str,
    ) -> str:
        # This check exists to make sure that the name is an actual event, otherwise other roles
        # that are non-events can't be added.
        if event_name not in [e.name for e in src.discord.globals.EVENT_INFO]:
            return f"`{event_name}` is not an event!"

        if self.fetch_role(event_name):
            return f"Role for `{event_name}` has already been added."

        await self.create_event_role(interaction, event_name)

        return f"Role for `{event_name}` has been added."

    async def disable_role(self, event_name: str) -> str:
        # This check exists to make sure that the name is an actual event, otherwise other roles
        # that are non-events can't be added.
        if event_name not in [e.name for e in src.discord.globals.EVENT_INFO]:
            return f"`{event_name}` is not an event!"

        potential_role = self.fetch_role(event_name)

        if not potential_role:
            return f"Role for `{event_name}` has already been deleted."

        potential_role.is_bot_managed
        await potential_role.delete()
        return f"Role for `{event_name}` has been deleted."

    @event_commands_group.command(
        name="role",
        description="Add or remove an event's role.",
    )
    @app_commands.checks.has_any_role(ROLE_STAFF, ROLE_VIP)
    @app_commands.describe(
        event_name="The name of the event.",
    )
    async def event_role(
        self,
        interaction: discord.Interaction,
        mode: EnableDisable,
        event_name: str,
    ):
        if mode == "enable":
            status = await self.enable_role(interaction, event_name)
            return await interaction.response.send_message(status)
        else:
            status = await self.disable_role(event_name)
            return await interaction.response.send_message(status)

    @event_batch_commands_group.command(
        name="role",
        description="Add or remove multiple events' role.",
    )
    @app_commands.checks.has_any_role(ROLE_STAFF, ROLE_VIP)
    @app_commands.describe(
        event_names_csv_list="A comma separated list of all event roles to add.",
    )
    async def event_batch_role(
        self,
        interaction: discord.Interaction,
        mode: EnableDisable,
        event_names_csv_list: str,
    ):
        event_name_list = event_names_csv_list.split(",")

        responses = []
        for event_name in event_name_list:
            try:
                if mode == "enable":
                    status_str = await self.enable_role(interaction, event_name)
                else:
                    status_str = await self.disable_role(event_name)
            except DiscordException as e:
                status_str = str(e)
            responses.append(status_str)

        await interaction.response.send_message("\n".join(responses))

    @event_commands_group.command(
        name="remove",
        description="Removes an event's availability and optionally, its role from all users.",
    )
    @app_commands.checks.has_any_role(ROLE_STAFF, ROLE_VIP)
    @app_commands.describe(
        event_name="The name of the event to remove.",
        delete_role="Whether to delete the event role from all users. 'no' allows role to remain.",
    )
    async def event_remove(
        self,
        interaction: discord.Interaction,
        event_name: str,
        delete_role: Literal["no", "yes"] = "no",
    ):
        # Check for staff permissions
        commandchecks.is_staff_from_ctx(interaction)

        return await interaction.response.send_message(
            "This command has been temporarily disabled as it gets reworked. If you want to remove"
            f"the event role, please use `/{self.event_role.qualified_name} disable`.",
        )

        # Send user notice that process has begun
        await interaction.response.send_message(
            f"{EMOJI_LOADING} Attempting to remove the `{event_name}` event...",
        )

        # Check to make sure event has previously been added
        event = next(
            (e for e in src.discord.globals.EVENT_INFO if e.name == event_name),
            None,
        )

        # Check to see if role exists on server
        server = self.bot.get_guild(env.server_id)
        potential_role = discord.utils.get(server.roles, name=event_name)

        # If staff member has selected to delete role from all users, delete role entirely
        if potential_role:
            if delete_role == "yes":
                if not event:
                    return await interaction.edit_original_response(
                        content=f"The event `{event_name}` role was found on the Discord server,"
                        "but was partially deleted. However, to prevent deletion of"
                        "incorrect/potentially privileged roles, no action was taken. To clean up"
                        "the role, go into the Discord server's role settings, and delete the role"
                        "manually.",
                    )
                await potential_role.delete()
                await event.delete()
                src.discord.globals.EVENT_INFO.remove(event)
                return await interaction.edit_original_response(
                    content=f"The `{event_name}` role was completely deleted from the server. All"
                    "members with the role no longer have it.",
                )
            if not event:
                return await interaction.edit_original_response(
                    content=f"The `{event_name}` event was previously deleted partially. There still "
                    "exists a role for the event.\n\nTo delete the role entirely, go into the"
                    "Discord server's role settings and delete the role manually.",
                )

        if not event:
            # If no event in list and no role exists on server
            return await interaction.edit_original_response(
                content=f"The `{event_name}` event does not exist.",
            )

        # Complete operation of removing event
        await event.delete()
        src.discord.globals.EVENT_INFO.remove(event)

        # Notify staff member of completion
        if not potential_role:
            await interaction.edit_original_response(
                content=f"The `{event_name}` event was deleted entirely. No role for the event was"
                "found, so no role was deleted.",
            )
        if delete_role == "no":
            await interaction.edit_original_response(
                content=f"The `{event_name}` event was deleted partially. Users who have the role"
                "currently will keep it, but new members can not access the role.\n\nTo delete the"
                "role entirely, go into the Discord server's role settings and delete the role"
                "manually.",
            )


async def setup(bot: PiBot):
    await bot.add_cog(StaffEvents(bot))

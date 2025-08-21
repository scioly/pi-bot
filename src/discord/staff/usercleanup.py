import asyncio
import logging
from collections.abc import Sequence

import discord
from discord import AllowedMentions, Member, Role, app_commands, ui
from discord.errors import Forbidden, HTTPException
from discord.ext import commands
from typing_extensions import Self

from bot import PiBot
from env import env
from src.discord.globals import (
    DISCORD_DEFAULT_INVITE_ENDING,
    DISCORD_LONG_TERM_RATE_LIMIT,
    EMOJI_LOADING,
    ROLE_MR,
    ROLE_STAFF,
    ROLE_VIP,
)


class UnconfirmedCleanupCancel(ui.View):
    """
    A View for showing progress on cleaning up unconfirmed users.

    Includes a cancellation button to cancel operation early. Note that this
    entire operation is not atomic and any users that were kicked prior to
    cancellation will not be reverted.
    """

    def __init__(
        self,
        interaction: discord.Interaction,
        initiator: discord.User | discord.Member,
        member_role: Role,
        total_member_count: int,
        members: Sequence[Member],
    ):
        super().__init__(timeout=None)
        self.initiator = initiator
        self.cancel_flag = asyncio.Event()
        self.task = asyncio.create_task(
            self.task_handler(
                interaction,
                self.cancel_flag,
                member_role,
                total_member_count,
                members,
            ),
        )

    async def interaction_check(
        self,
        interaction: discord.Interaction[discord.Client],
    ) -> bool:
        if interaction.user == self.initiator:
            return True
        await interaction.response.send_message(
            f"The command was initiated by {self.initiator.mention}",
            ephemeral=True,
        )
        return False

    @discord.ui.button(label="Cancel", style=discord.ButtonStyle.red)
    async def cancel(
        self,
        interaction: discord.Interaction,
        button: discord.ui.Button[Self],
    ) -> None:
        button.disabled = True
        button.label = "Cancelling ..."
        self.cancel_flag.set()

        async def handle_done():
            await interaction.edit_original_response(content="Cancelled")

        self.task.add_done_callback(lambda _: asyncio.create_task(handle_done()))

    async def task_handler(
        self,
        interaction: discord.Interaction[discord.Client],
        cancel_event: asyncio.Event,
        member_role: Role,
        total_member_count: int,
        members: Sequence[Member],
    ):
        """
        In charge of processing all users to kick. Passes message rendering to another async task.

        Can be cancelled via button action. If cancelled, the coroutine is gracefully terminated.
        """
        if not interaction.command:
            raise Exception("Handler was not invoked via command")
        if not interaction.guild:
            raise Exception("Command should be invoked within a server")
        chunk_size = 100
        members_processed = 0
        members_failed: list[Member] = []
        lock = asyncio.Lock()

        end_event = asyncio.Event()

        async def progress_updater(end_signal: asyncio.Event):
            while not end_signal.is_set():
                async with lock:
                    progress_message = "{} {}/~{} users processed".format(
                        EMOJI_LOADING,
                        members_processed,
                        total_member_count,
                    )
                    for failed_member in members_failed:
                        progress_message += f"\n{failed_member.mention}"
                    final_message = asyncio.create_task(
                        interaction.edit_original_response(
                            content=progress_message,
                            view=self,
                            allowed_mentions=AllowedMentions.none(),
                        ),
                    )
                await asyncio.sleep(DISCORD_LONG_TERM_RATE_LIMIT)

            if final_message:
                await final_message

        ui_updater = asyncio.create_task(progress_updater(end_event))

        for member_chunk in discord.utils.as_chunks(members, chunk_size):
            failed_members: list[Member] = []
            extra_processed = None
            for i, member in enumerate(member_chunk):
                if cancel_event.is_set():
                    extra_processed = i
                    break
                if member_role not in member.roles:
                    try:
                        await member.kick(
                            reason="Server cleanup. Please rejoin the server if available (discord.gg/{})".format(
                                DISCORD_DEFAULT_INVITE_ENDING,
                            ),
                        )
                    except (Forbidden, HTTPException) as e:
                        logging.warning(
                            "{}: Failed to kick user @{}: {}",
                            interaction.command.qualified_name,
                            member.name,
                            e,
                        )
                        failed_members.append(member)

            async with lock:
                members_processed += (
                    extra_processed if extra_processed else len(member_chunk)
                )
                members_failed.extend(failed_members)
            if cancel_event.is_set():
                break

        end_event.set()
        await ui_updater

        users_with_unconfirmed_role = sum(
            [
                1
                async for member in interaction.guild.fetch_members(limit=None)
                if member_role not in member.roles
            ],
        )

        progress_message = "Completed"
        if cancel_event.is_set():
            progress_message = "Cancelled"
        for failed_member in members_failed:
            progress_message += f"\n{failed_member.mention}"
        if users_with_unconfirmed_role > 0:
            progress_message += (
                "\nThere exist {} user(s) that does not have the {} role".format(
                    users_with_unconfirmed_role,
                    member_role.name,
                )
            )

        return await interaction.edit_original_response(
            content=progress_message,
            allowed_mentions=AllowedMentions.none(),
        )


class UserCleanup(commands.Cog):
    def __init__(self, bot: PiBot):
        self.bot = bot

    cleanup_command_group = app_commands.Group(
        name="cleanup",
        description="Staff commands to help facilitate easy tidying",
        guild_ids=env.slash_command_guilds,
        default_permissions=discord.Permissions(manage_messages=True),
    )

    @cleanup_command_group.command(
        name="unconfirmed",
        description="Kicks any person with the old Unconfirmed role with. Meant to be run one time.",
    )
    @app_commands.checks.has_any_role(ROLE_STAFF, ROLE_VIP)
    async def remove_unconfirmed_users(self, interaction: discord.Interaction):
        """
        Kicks any users that do not have the Member role in the current server
        the command was invoked.

        Includes a cancellation button to cancel operation early. Note that this
        entire operation is not atomic and any users that were kicked prior to
        cancellation will not be reverted.
        """
        if not interaction.command:
            raise Exception("Handler was not invoked via command")
        if not interaction.guild:
            raise Exception("Command should be invoked within a server")

        member_role = discord.utils.get(interaction.guild.roles, name=ROLE_MR)

        if not member_role:
            raise Exception(
                f"Could not find role `{ROLE_MR}`. Please make sure it exists and the bot has adequate permissions.",
            )

        await interaction.response.send_message(
            view=UnconfirmedCleanupCancel(
                interaction,
                interaction.user,
                member_role,
                total_member_count=interaction.guild.member_count
                or len(interaction.guild.members),
                members=interaction.guild.members,
            ),
        )


async def setup(bot: PiBot):
    await bot.add_cog(UserCleanup(bot))

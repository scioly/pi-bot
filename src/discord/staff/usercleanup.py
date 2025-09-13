import asyncio
import logging
from asyncio.locks import Event

import discord
from discord import AllowedMentions, Member, app_commands, ui
from discord.errors import HTTPException
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
from src.discord.staffcommands import Confirm


class UnconfirmedCleanupCancel(ui.View):
    """
    A View for showing progress on cleaning up unconfirmed users.

    Includes a cancellation button to cancel operation early. Note that this
    entire operation is not atomic and any users that were kicked prior to
    cancellation will not be reverted.
    """

    def __init__(
        self,
        initiator: discord.User | discord.Member,
    ):
        super().__init__(timeout=None)
        self.initiator = initiator
        self.cancel_flag = asyncio.Event()

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
        _: discord.Interaction,
        button: discord.ui.Button[Self],
    ) -> None:
        button.disabled = True
        button.label = "Cancelling ..."
        self.cancel_flag.set()

    def get_cancel_event(self) -> Event:
        return self.cancel_flag


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
    @app_commands.checks.bot_has_permissions(kick_members=True, send_messages=True)
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

        view = Confirm(
            interaction.user,
            "Cleanup operation was cancelled. All unconfirmed users should still be on the server.",
        )
        await interaction.response.send_message(
            "Please confirm that you want to purge all non-members from the server.",
            view=view,
        )

        await view.wait()

        total_member_count = interaction.guild.member_count or len(
            interaction.guild.members,
        )

        cancel_progress_view = UnconfirmedCleanupCancel(interaction.user)
        chunk_size = 100
        members_processed = 0
        members_kicked = 0
        members_failed: list[Member] = []
        lock = asyncio.Lock()

        cancel_event = cancel_progress_view.get_cancel_event()
        end_progress_event = asyncio.Event()

        async def progress_updater(end_signal: asyncio.Event):
            if end_signal.is_set():
                return
            while True:
                async with lock:
                    progress_message = (
                        "{} {}/~{} users processed\n{}/{} users kicked".format(
                            EMOJI_LOADING,
                            members_processed,
                            total_member_count,
                            members_kicked,
                            members_processed,
                        )
                    )
                    for failed_member in members_failed:
                        progress_message += f"\n{failed_member.mention}"
                    final_message = asyncio.create_task(
                        interaction.edit_original_response(
                            content=progress_message,
                            allowed_mentions=AllowedMentions.none(),
                            view=cancel_progress_view,
                        ),
                    )
                try:
                    await asyncio.wait_for(
                        end_signal.wait(),
                        timeout=DISCORD_LONG_TERM_RATE_LIMIT,
                    )
                    break
                except asyncio.TimeoutError:
                    pass

            if final_message:
                await final_message

        ui_updater = asyncio.create_task(progress_updater(end_progress_event))

        def member_predicate(member: discord.Member) -> bool:
            return not member.bot and member_role not in member.roles

        embed_message = discord.Embed(
            title="You have been kicked in the Scioly.org server.",
            color=discord.Color.brand_red(),
            description=(
                "You were kicked from the Scioly.org server since "
                "you did not fill out the onboarding survey "
                "fully. You are free to rejoin the server at "
                "your earliest convenience "
                f"(https://discord.gg/{DISCORD_DEFAULT_INVITE_ENDING}).",
            ),
        )
        for member_chunk in discord.utils.as_chunks(
            interaction.guild.members,
            chunk_size,
        ):
            failed_members: list[Member] = []
            extra_processed = None
            kicked_count = 0
            for i, member in enumerate(member_chunk):
                if cancel_event.is_set():
                    extra_processed = i
                    break
                if not member_predicate(member):
                    continue
                try:
                    # We cannot send a message to the user after they are
                    # kicked, so we must send one first before we call
                    # kick()
                    await member.send(
                        "Notice from the Scioly.org server:",
                        embed=embed_message,
                    )
                except HTTPException as e:
                    logging.warning(
                        "{}: Could not send message notify user @{}: {}",
                        interaction.command.qualified_name,
                        member.name,
                        e,
                    )
                try:
                    await member.kick(
                        reason="Server cleanup - Did not fill out onboarding survey",
                    )
                    kicked_count += 1
                except HTTPException as e:
                    logging.error(
                        "{}: Failed to kick user @{}: {}",
                        interaction.command.qualified_name,
                        member.name,
                        e,
                    )
                    failed_members.append(member)
                await asyncio.sleep(DISCORD_LONG_TERM_RATE_LIMIT)

            async with lock:
                members_processed += (
                    extra_processed if extra_processed else len(member_chunk)
                )
                members_kicked += kicked_count
                members_failed.extend(failed_members)
            if cancel_event.is_set():
                break

        end_progress_event.set()
        await ui_updater

        users_without_member_role = sum(
            [
                1
                async for member in interaction.guild.fetch_members(limit=None)
                if member_predicate(member)
            ],
        )

        progress_message = "Operation completed"
        if cancel_event.is_set():
            progress_message = "Cancelled by initiator"
        progress_message += f"\nProcessed {members_processed} members"
        progress_message += f"\nKicked {members_kicked} members"
        if members_failed:
            progress_message += "\nFailed to process the following members:"
        for failed_member in members_failed:
            progress_message += f"\n- {failed_member.mention}"
        if users_without_member_role > 0:
            progress_message += (
                "\nThere exist {} user(s) that does not have the {} role".format(
                    users_without_member_role,
                    member_role.name,
                )
            )

        return await interaction.edit_original_response(
            content=progress_message,
            allowed_mentions=AllowedMentions.none(),
            view=None,
        )


async def setup(bot: PiBot):
    await bot.add_cog(UserCleanup(bot))

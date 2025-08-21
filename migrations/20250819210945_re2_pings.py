from typing import Annotated

from beanie import Document, Indexed, iterative_migration
from pydantic import BaseModel


class PhrasePing(BaseModel):
    phrase: str
    is_re2: bool


class NewPing(Document):
    user_id: Annotated[int, Indexed()]
    pings: list[PhrasePing]
    dnd: bool

    class Settings:
        name = "pings"


class OldPing(Document):
    user_id: Annotated[int, Indexed()]
    word_pings: list[str]
    dnd: bool

    class Settings:
        name = "pings"


class Forward:
    @iterative_migration()
    async def str_to_word_ping_model(
        self,
        input_document: OldPing,
        output_document: NewPing,
    ):
        output_document.pings = [
            PhrasePing(phrase=word, is_re2=False) for word in input_document.word_pings
        ]


class Backward:
    @iterative_migration()
    async def str_to_word_ping_model(
        self,
        input_document: NewPing,
        output_document: OldPing,
    ):
        output_document.word_pings = [
            ping.phrase for ping in input_document.pings if not ping.is_re2
        ]

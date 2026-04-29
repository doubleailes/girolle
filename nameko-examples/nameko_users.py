"""
Minimal Python Nameko mirror of `examples/src/event_emitter.rs`.

Exposes `users.create_user(name)` and emits a `user_created` event on the
`users.events` topic exchange. Use it to verify cross-language wire
compatibility with the Rust `event_observer` example.

Run from this directory:

    RABBITMQ_USER=... \
    RABBITMQ_PASSWORD=... \
    RABBITMQ_HOST=... \
    nameko run --config config.yml nameko_users

Then trigger from the project root:

    cargo run --example cli_sender -- users create_user Girolle
"""

from nameko.events import EventDispatcher
from nameko.rpc import rpc


class UsersService:
    name = "users"
    dispatch = EventDispatcher()

    @rpc
    def create_user(self, name):
        self.dispatch("user_created", {"name": name})
        return "User {} created".format(name)

"""
Minimal Python Nameko mirror of `examples/src/event_observer.rs`.

Subscribes to `users.events / user_created` and prints each event as it
arrives. Use it to verify Rust-emitter → Python-subscriber wire
compatibility against the Rust `event_emitter` example.

Run from this directory:

    RABBITMQ_USER=... \
    RABBITMQ_PASSWORD=... \
    RABBITMQ_HOST=... \
    uv run nameko run --config ../staging/config.yml nameko_user_audit

Then trigger an emit (Rust side):

    cargo run --example event_emitter        # in another terminal
    cargo run --example cli_sender -- users create_user Girolle
"""

from nameko.events import event_handler


class UserAuditService:
    name = "user_audit"

    @event_handler("users", "user_created")
    def on_user_created(self, payload):
        print("[users.user_created] {}".format(payload))

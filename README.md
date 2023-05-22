# Lute V4

## Design Principles

- Event Sourced: The system's state is persisted as an event stream. All materialized indexes
  can be reconstructed to any point in time by replaying the event stream.
- Portable: The core component of the system is a monolith packaged as a single executable that
  can be run on any platform.
- Controllable: The system offers a rich set of control interfaces and configuration options.
- Malleable: The system is designed to be extended and modified.
- Polite: The crawler imposes minimal load on the target site.

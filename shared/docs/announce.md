# Announcement banner

Status: **Confirmed** in released mods of the old repository ("Config reloaded", "Config error - see log"); not
yet re-tested in game from this one. Conventions: [research notes](../../docs/research/README.md). Code:
`shared/src/announce.rs`.

## Question

How can a mod show a line of text in the game's top-of-screen banner, for example to confirm a config reload?

## Mechanism

`show_announcement(text)` pushes an `AnnounceNotification` onto
`CSMenuManImp.system_announce_view_model.notifications`:

- The message is a `MenuString` whose `allocated_string` is a `DLString` built with the game's own allocator
  ([alloc_hook.md](alloc_hook.md)); `static_string` stays null. The game frees the string itself when the
  notification ends, so it must come from that allocator.
- It does nothing when the menu manager or the allocator is not resolved yet, or when the string cannot be
  built.
- Call it from a game task (the reload task does): the singleton is taken as `instance_mut`, which is only
  sound while nothing else holds a reference to it, and game tasks run one after another.

Used by `reload::run` for the banner after a successful reload (when the mod's `ReloadBanner` is on) and for
the error banner (always shown).

## Open risks

The banner is a shared UI element: another mod pushing to the same queue is not synchronised with this one.

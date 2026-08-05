# systemd user units for the localfuzz pipeline

Reference copies of the `systemd --user` unit files actually deployed
on the dev machine at `~/.config/systemd/user/` (not consumed
directly from here — install by copying, see below). Paths inside
are absolute and specific to that machine's `$HOME`; adjust before
installing elsewhere.

- `vani-localfuzz-autostart.service` — runs `start.sh` at boot
  (`Type=oneshot`, `RemainAfterExit=yes`), which itself launches the
  two transient, sandboxed `vani-localfuzz-ollama`/
  `vani-localfuzz-harness` units. `ExecStop` runs `stop.sh`.
- `vani-localfuzz-refresh.service` + `.timer` — nightly (03:00, ±10min
  jitter) `refresh.sh`: merges `main`, rebuilds `vanic` so the fuzz
  harness stays current.
- `vani-localfuzz-digest.service` + `.timer` — nightly (06:00)
  `digest.py`: summarizes new findings into `DIGEST_LATEST.md`.

The harness/ollama pair's own units
(`vani-localfuzz-harness.service`, `vani-localfuzz-ollama.service`)
are NOT here — they're created transiently by `start.sh`'s own
`systemd-run --user --unit=...` invocations each time it runs, not
static unit files.

## Install (fresh machine)

```sh
cp tools/localfuzz/systemd/*.service tools/localfuzz/systemd/*.timer \
   ~/.config/systemd/user/
# edit the ExecStart= paths in each file if this checkout isn't at
# the same absolute path as the machine these were copied from
systemctl --user daemon-reload
systemctl --user enable --now vani-localfuzz-refresh.timer
systemctl --user enable --now vani-localfuzz-digest.timer
systemctl --user enable --now vani-localfuzz-autostart.service
loginctl enable-linger "$USER"   # so these start at boot, not just at login
```

`loginctl enable-linger` is required for boot-time start without an
interactive login session — without it, none of these fire until the
user actually logs in.

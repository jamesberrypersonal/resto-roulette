# Release Notes

## v0.2.0

- Pretty output now shows the transport mode used for each time estimate (e.g. `~8 min by walking`)
- Re-roll is now the default; use `--one-shot` / `-o` to pick once and exit
- Fixed bug where `--cache-ttl` CLI flag was incorrectly overridden by the config file value
- Removed unused code (dead geocoding method, unused error variant)

## v0.1.1

- Upgraded dependencies
- Updated config file to optionally include list path
- Fixed bug where drive-only restaurants under 30 min were incorrectly placed in the Far (30–60 min) bucket

## v0.1.0

- Initial release

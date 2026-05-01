# Troubleshooting

## Linux build fails with fontconfig pkg-config error

Install fontconfig development headers:

```bash
sudo apt-get update && sudo apt-get install -y libfontconfig1-dev
```

## Command not found: forecasts

Use the built binary path directly:

```bash
./target/release/forecasts --help
```

Or ensure your install location is in `PATH`.

## Invalid date parsing

Use the date format `YYYY-MM-DD` consistently in YAML input files.

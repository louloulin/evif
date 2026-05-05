# Homebrew Formula for EVIF

## Installation

### Option 1: From this formula (local tap)

```bash
# Create a local tap
brew tap evif-io/evif <path-to-this-directory>

# Install
brew install evif-io/evif/evif
```

### Option 2: From GitHub Releases (recommended for users)

```bash
brew install evif-io/evif/tap/evif
```

Or add our tap first:

```bash
brew tap evif-io/evif https://github.com/evif/evif
brew install evif
```

### Option 3: Build from source

```bash
brew install --build-from-source evif-io/evif/evif
```

## Post-Installation

1. **Start the EVIF REST server** (optional, for HTTP access):
   ```bash
   evif rest serve &
   ```

2. **Connect to Claude Desktop**:
   ```bash
   evif connect claude
   ```

3. **Verify installation**:
   ```bash
   evif health
   ```

## Uninstallation

```bash
brew uninstall evif
brew untap evif-io/evif
rm -rf ~/.evif
```

## Updating

```bash
brew upgrade evif
```

## Requirements

- macOS 12.0+ or Linux
- OpenSSL 3.x (installed automatically via Homebrew)

## Binary Distribution

The formula downloads pre-built binaries from GitHub Releases for:
- macOS (Intel and Apple Silicon)
- Linux (x86_64 and aarch64)

If you need Linux support in the official tap, see the `linux.yml` workflow in `.github/workflows/`.

## Development

To test formula changes locally:

```bash
# Install from local formula
brew install --debug ./homebrew-formula/evif.rb

# Audit for issues
brew audit --new-package ./homebrew-formula/evif.rb

# Test installation
brew test ./homebrew-formula/evif.rb
```

## License

EVIF is MIT or Apache-2.0 licensed. See [LICENSE](LICENSE).
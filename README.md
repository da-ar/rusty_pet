# Rusty Pet

A command-line utility for interacting with the SurePet API. Monitor your pets and control pet flaps through both interactive menus and direct CLI commands.

## Features

- 🐱 **Pet Management**: List pets, check locations, and set pet locations
- 🚪 **Basic Device Control**: Lock/unlock pet flaps and check device status
- 📊 **History Access**: View feeding, drinking, and activity history
- 🎯 **Two Modes**: Interactive menu-driven interface or direct CLI commands
- 🔐 **Secure Authentication**: Token-based auth with automatic persistence

## Installation

### From Source

```bash
git clone https://github.com/da-ar/rusty_pet.git
cd rusty_pet
cargo build --release
```

The binary will be available at `target/release/rusty_pet`.

### Prerequisites

- Rust 2021 Edition or later
- SurePet account with API access

## Authentication

Rusty Pet supports multiple authentication methods with automatic fallback:

1. **Environment Variable** (highest priority):
   ```bash
   export SUREHUB_TOKEN="your_token_here"
   rusty_pet status
   ```

2. **Saved Token File**: Tokens are automatically saved to `~/.rusty_pet_token` after interactive login

3. **Interactive Login**: If no token is found, you'll be prompted to log in

### First Time Setup

Run any command to trigger the authentication flow:

```bash
rusty_pet status
```

You'll be prompted for your SurePet credentials, and the token will be saved for future use.

## Usage

Rusty Pet operates in two modes:

### Interactive Mode

Run without any commands to enter the interactive menu:

```bash
rusty_pet
```

This provides a user-friendly menu system for browsing pets, checking status, and managing basic settings.

### CLI Mode (Headless)

Execute specific commands directly:

```bash
# Check system status and devices
rusty_pet status

# List all pets
rusty_pet list

# List pets with filters
rusty_pet list --name "Fluffy" --location inside

# Set a pet's location
rusty_pet set-location "Fluffy" inside
rusty_pet set-location "Max" outside

# Control pet flaps
rusty_pet lock "Pet Door"
rusty_pet unlock "Pet Door"
rusty_pet lock-in "Pet Door"    # Pets can exit but not enter
rusty_pet lock-out "Pet Door"   # Pets can enter but not exit

# View history
rusty_pet feeding-history "Fluffy" --range week
rusty_pet drinking-history "Max" --range today
rusty_pet activity-history "Fluffy" --range month
```

## Commands Reference

### Pet Management

```bash
# List all pets
rusty_pet list

# List pets with filters
rusty_pet list --name "Flu" --location inside --sort activity

# Set pet location
rusty_pet set-location "Fluffy" inside
rusty_pet set-location "Max" outside

# Mark pet as indoor/outdoor
rusty_pet set-indoor "Fluffy"
rusty_pet set-outdoor "Max"
```

### Device Control

```bash
# Lock pet flap completely (no access)
rusty_pet lock "Pet Door"

# Unlock pet flap (free access)
rusty_pet unlock "Pet Door"

# Keep pets inside (can exit, can't enter)
rusty_pet lock-in "Pet Door"

# Keep pets outside (can enter, can't exit)
rusty_pet lock-out "Pet Door"

# Set curfew times
rusty_pet set-curfew "Pet Door" --lock-time 22:00 --unlock-time 06:00

# Disable curfew
rusty_pet set-curfew "Pet Door" --disable
```

### History & Data

```bash
# Get feeding history
rusty_pet feeding-history "Fluffy" --range today
rusty_pet feeding-history "Fluffy" --range week
rusty_pet feeding-history "Fluffy" --range 2024-01-01,2024-01-31

# Get drinking history
rusty_pet drinking-history "Max" --range month

# Get activity history
rusty_pet activity-history "Fluffy" --range week
```

### System Management

```bash
# Check system status and device information
rusty_pet status

# Clear saved token (logout)
rusty_pet logout

# Reset configuration to defaults
rusty_pet reset-config --yes
```

## Output Formats

### Human-Readable (Default)

```bash
rusty_pet list
```

Provides formatted, colorized output perfect for terminal viewing.

### JSON Output

```bash
rusty_pet list --json
```

Machine-readable JSON format ideal for scripting and automation.

### Verbose Output

```bash
rusty_pet status --verbose
```

Includes debug information and detailed logging.

## Configuration

Configuration is stored in `src/assets/client_config.toml` and includes API endpoints and request settings.

## Error Handling

Rusty Pet provides clear error messages and suggestions:

```bash
# Invalid pet name
rusty_pet set-location "NonExistent" inside
# Error: No pet found with name or ID 'NonExistent'

# Multiple matches
rusty_pet set-location "Fl" inside  
# Error: Multiple pets match 'Fl': Fluffy (ID: 123), Flint (ID: 456). Please be more specific or use the pet ID.
```

## Examples

### Daily Pet Management

```bash
#!/bin/bash
# Check pet status
rusty_pet status

# List all pets and their locations
rusty_pet list

# Set pet locations
rusty_pet set-location "Fluffy" inside
rusty_pet set-location "Max" outside

# Check recent feeding activity
rusty_pet feeding-history "Fluffy" --range today
```

### Device Management

```bash
#!/bin/bash
# Lock pet flap for the night
rusty_pet lock "Main Door"

# Set up evening curfew
rusty_pet set-curfew "Main Door" --lock-time 22:00 --unlock-time 06:00

# Check device status
rusty_pet status --verbose
```

## Troubleshooting

### Authentication Issues

```bash
# Clear saved token and re-authenticate
rusty_pet logout
rusty_pet status  # Will prompt for new login
```

### Connection Problems

```bash
# Check system status with verbose output
rusty_pet status --verbose
```

### Configuration Issues

```bash
# Reset to default configuration
rusty_pet reset-config --yes
```

## Development

### Building from Source

```bash
git clone https://github.com/da-ar/rusty_pet.git
cd rusty_pet
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Development Mode

```bash
cargo run -- status --verbose
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Support

- 📖 Check the command help: `rusty_pet --help`
- 🐛 Report issues on GitHub
- 💡 Request features through GitHub issues

## Changelog

### v0.1.0
- Initial release
- Interactive and CLI modes
- Pet and device management
- Basic history access
- Token-based authentication
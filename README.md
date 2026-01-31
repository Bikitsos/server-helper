# Server Helper

A terminal user interface (TUI) application for Windows Server administration tasks. Built with Rust using the Ratatui framework.

## Features

### Package Management
- **Check Winget Status** - Verify if Windows Package Manager (winget) is installed
- **Install Winget** - Install winget on Windows Server (downloads and installs all required dependencies)

### VPN/Networking
- **Check NetBird Status** - Verify if NetBird VPN client is installed
- **Install NetBird** - Install NetBird via winget or fallback to direct installer

### Server Roles and Features
- **Backup Server Roles & Features** - Export all installed Windows Server roles and features to an XML file for later restoration
- **Restore Server Roles & Features** - Browse and select a backup file to restore server roles and features on a new server

## Requirements

- Windows Server 2016 or later
- Administrator privileges (required for installing software and managing server roles)
- [Rust](https://www.rust-lang.org/tools/install) 1.70 or later (for building from source)

## Installation

### From Source

```bash
git clone https://github.com/yourusername/server-helper.git
cd server-helper
cargo build --release
```

The executable will be available at `target/release/server-helper.exe`

### Pre-built Binary

Download the latest release from the Releases page.

## Usage

Run the application as Administrator:

```bash
server-helper.exe
```

### Navigation

| Key | Action |
|-----|--------|
| Up/Down or j/k | Navigate menu items |
| Enter | Select/Confirm |
| Esc | Cancel/Go back |
| q | Quit |
| Backspace | Parent directory (in file browser) |

## Backup and Restore

### Backup Location

Server role backups are saved to:
```
Documents\ServerBackups\ServerRoles_<timestamp>.xml
Documents\ServerBackups\InstalledFeatures_<timestamp>.txt
```

### Manual Restore

If you prefer to restore manually via PowerShell:

```powershell
Import-Clixml 'path\to\ServerRoles_timestamp.xml' | Where-Object {$_.Installed} | Install-WindowsFeature
```

## Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release
```

## Running

```bash
cargo run
```

## License

MIT

## Testing

```bash
cargo test
```

## Project Structure

```
serverHelper/
├── Cargo.toml    # Project manifest and dependencies
├── src/
│   └── main.rs   # Application entry point
└── README.md
```

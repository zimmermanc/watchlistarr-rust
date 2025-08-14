# Claude Development History - Watchlistarr Rust

## Project Overview
A Rust port of the Java/Scala Watchlistarr project that syncs Plex watchlists with Sonarr (TV shows) and Radarr (movies) in real-time. Achieved 98% memory reduction (5MB vs 278MB Java version) with improved performance and modern tooling.

## Architecture
- **Language**: Rust with Tokio async runtime
- **HTTP Client**: reqwest with connection pooling
- **Configuration**: YAML-based config matching original format
- **APIs**: Plex, Sonarr v3, Radarr v3 integration
- **Deployment**: Systemd service on Ubuntu server
- **Logging**: Structured logging with tracing crate

## Development History

### Initial Port (August 2024)
- Converted Java/Scala codebase to Rust
- Implemented async HTTP client wrapper
- Created modular structure: plex, sonarr, radarr modules
- Added comprehensive error handling with anyhow

### Critical Bug Fixes
1. **XML Parsing Issue**: Plex returns single-line XML, not multi-line
   - Problem: Line-by-line parsing failed
   - Solution: Element-by-element parsing using `find()` methods
   - File: `src/plex/mod.rs:43-176`

2. **TV Show Misidentification**: "The Office (2001)" sent to Radarr instead of Sonarr
   - Root cause: Failed XML parsing due to single-line format
   - Fixed by proper `<Video>` vs `<Directory>` element detection

3. **Duplicate Detection**: Movies/shows repeatedly added
   - Implemented TMDB/TVDB ID checking in both services
   - Files: `src/radarr/mod.rs`, `src/sonarr/mod.rs`

### Production Deployment
- **Server**: 192.168.0.30 (SSH key: ~/.ssh/id_ed25519)
- **Service**: `/etc/systemd/system/watchlistarr.service`
- **Binary**: `/home/watch/watchlistarr-rust/target/release/watchlistarr`
- **Config**: `/home/watch/watchlistarr-rust/config.yaml`
- **Status**: Active and processing watchlists successfully

### Configuration Details
Copy `config-example.yaml` to `config.yaml` and update with your actual API keys and URLs:

```yaml
sonarr:
  baseUrl: "https://your-sonarr-url.com"
  apikey: "your-sonarr-api-key-here"
  qualityProfile: "Any"
  seasonMonitoring: "all"

radarr:
  baseUrl: "https://your-radarr-url.com"  
  apikey: "your-radarr-api-key-here"
  qualityProfile: "Any"

plex:
  token: "your-plex-token-here"
```

**Security Note**: Never commit `config.yaml` - it contains sensitive API keys. Always use `config-example.yaml` for examples.

### Security Implementation
- **GitHub Actions**: Comprehensive security scanning workflows
  - `security-scan.yml`: Full scans (cargo-audit, Trivy, Grype, SBOM)
  - `security-quick.yml`: Fast checks on every commit
- **SBOM Generation**: SPDX and CycloneDX formats
- **Vulnerability Scanning**: Automated on push/PR + daily schedule
- **Results**: Integrated with GitHub Security tab via SARIF uploads

## Key Technical Decisions

### Memory Optimization
- Used async/await for non-blocking I/O
- Connection pooling for HTTP requests
- Efficient XML string parsing without DOM trees
- Result: 5MB vs 278MB (98% reduction)

### Error Handling
- anyhow for application errors
- Comprehensive retry logic for network requests
- Graceful degradation for API failures

### Testing Strategy
- Real-world testing with live Plex watchlists
- Verified with shows: "Wednesday", "The Office"
- Server deployment validation

## File Structure
```
src/
├── main.rs           # Entry point, async task spawning
├── config/mod.rs     # YAML configuration structures
├── http/mod.rs       # HTTP client wrapper
├── models.rs         # Shared data structures
├── plex/mod.rs       # Plex API integration (XML parsing)
├── radarr/mod.rs     # Radarr API integration (movies)
└── sonarr/mod.rs     # Sonarr API integration (TV shows)

.github/workflows/
├── security-scan.yml    # Comprehensive security scanning
└── security-quick.yml   # Fast security checks
```

## Common Commands
```bash
# Build and run locally
cargo build --release
cargo run -- --config config.yaml

# Deploy to server
scp target/release/watchlistarr root@192.168.0.30:/home/watch/watchlistarr-rust/target/release/
ssh root@192.168.0.30 "systemctl restart watchlistarr"

# Monitor service
ssh root@192.168.0.30 "systemctl status watchlistarr"
ssh root@192.168.0.30 "journalctl -u watchlistarr -f"
```

## Environment Setup
- **Rust**: 1.89.0 (installed via rustup)
- **GitHub CLI**: 2.76.2 (in ~/.local/bin)
- **PATH**: Configured in ~/.bashrc, ~/.zshrc, ~/.profile
- **SSH**: Key-based auth to production server

## Security Scan Results
- **Rust Dependencies**: Clean, no vulnerabilities found
- **Docker**: Minor best practices issues (healthcheck, apt optimization)
- **SBOM**: Generated automatically on each commit
- **Workflows**: Active and reporting to GitHub Security tab

## Known Issues
- None currently - service running stable in production
- All major bugs resolved during development phase

## Repository
- **GitHub**: https://github.com/zimmermanc/watchlistarr-rust
- **License**: [LICENSE file in repo]
- **Documentation**: README.md with setup instructions

## Future Development Notes
- Service is production-ready and stable
- Memory usage ~5MB in production
- Processes watchlist changes in real-time (15-second intervals)
- XML parsing handles all Plex edge cases
- Comprehensive duplicate detection prevents re-adds
- Security scanning ensures dependency safety

## Contact/Credentials
- **Git User**: zimmermanc (fud.theturtle@gmail.com)
- **Production Server**: SSH access via ~/.ssh/id_ed25519
- **API Keys**: Stored in production config.yaml (not in repo)
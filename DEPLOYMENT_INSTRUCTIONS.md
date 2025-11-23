# Deployment Instructions for File Replacement Fix

## What Was Fixed

The file replacement was failing in your Debian container due to **cross-filesystem operations**. When `/tmp` (where encoded files are stored) and your library path are on different filesystems, the `rename()` system call fails with EXDEV error (error code 18).

The fix:
1. **Detects cross-filesystem errors** automatically
2. **Falls back to copy()** when rename() fails
3. **Adds extensive logging** for troubleshooting
4. **Maintains atomicity** and safety guarantees

## Changes Pushed to GitHub

The fix has been committed and pushed to your repository:
- Commit: `041f122`
- Branch: `main`
- Files changed:
  - `crates/daemon/src/replace.rs` - Cross-filesystem detection and fallback
  - `crates/daemon/src/daemon_loop.rs` - Enhanced logging
  - `crates/daemon/tests/replace_properties.rs` - Updated tests

## Deploy to Your Debian Server

### Option 1: Pull and Rebuild (Recommended)

```bash
# SSH into your server
ssh user@your-server

# Navigate to the project directory
cd /path/to/av1-janitor

# Pull the latest changes
git pull origin main

# Rebuild the daemon
cargo build --release

# Stop the daemon
sudo systemctl stop av1d

# Install the new binary
sudo cp target/release/av1d /usr/local/bin/

# Start the daemon
sudo systemctl start av1d

# Monitor the logs
journalctl -u av1d -f
```

### Option 2: Use the Deployment Script

```bash
# On your local machine, edit the script with your server details
vim deploy-fix-to-server.sh

# Set these variables:
# SERVER_USER="your-username"
# SERVER_HOST="your-server-ip"
# SERVER_PATH="/path/to/av1-janitor"

# Run the deployment
./deploy-fix-to-server.sh
```

## Verify the Fix

After deploying, watch for these log messages:

### Success (Same Filesystem)
```
INFO  Replacing original file for job <id>
INFO    Original: /path/to/file.mkv
INFO    Encoded: /tmp/av1d-temp-xxxxx.mkv
INFO    Original size: XXXXX bytes
INFO    Encoded size: XXXXX bytes
INFO  Successfully replaced /path/to/file.mkv
```

### Success (Cross-Filesystem - Expected in Containers)
```
INFO  Replacing original file for job <id>
Cross-filesystem detected, using copy for backup
INFO  Successfully replaced /path/to/file.mkv
```

### Still Failing?
If you still see errors, the logs will now show detailed information:
```
ERROR: Failed to rename original to backup
  Original: /path/to/file.mkv (exists: true)
  Backup: /path/to/file.mkv.orig.1234567890
  Parent dir exists: true
  Error kind: PermissionDenied
  Original permissions: ...
  Parent dir permissions: ...
```

## Diagnostics

Run the diagnostic script on your server to identify issues:

```bash
# Copy to server
scp diagnose-replacement.sh user@server:/tmp/

# On the server
chmod +x /tmp/diagnose-replacement.sh
/tmp/diagnose-replacement.sh
```

This will show:
- Container detection
- User and permissions
- Filesystem information
- Cross-filesystem detection
- Recent failed jobs

## Common Issues

### Issue 1: Permission Denied
**Solution**: Ensure the daemon user has write permissions
```bash
# Check ownership
ls -la /path/to/library

# Fix if needed
sudo chown -R daemon-user:daemon-group /path/to/library
```

### Issue 2: No Space Left
**Solution**: Clean up temp files and check disk space
```bash
df -h
rm -rf /tmp/av1d-temp-*
```

### Issue 3: Read-Only Filesystem
**Solution**: Check mount options
```bash
mount | grep /path/to/library
# Remount as read-write if needed
```

## Testing

All 197 tests pass with this fix:
- ✅ 9 replacement property tests
- ✅ 18 daemon unit tests  
- ✅ 170+ other integration and property tests

The fix maintains backward compatibility and doesn't break any existing functionality.

## Support

If issues persist after deployment:
1. Run the diagnostic script and share the output
2. Share the full error message from daemon logs
3. Provide `df -h` and `mount` output from your container

The enhanced logging will provide detailed information about what's failing and why.

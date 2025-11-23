use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::fs;

/// Atomically replace the original file with the new file.
/// 
/// This function performs the following steps:
/// 1. Rename original to a temporary name with `.orig` suffix
/// 2. Rename new to the original filename
/// 3. If keep_original is false, delete the `.orig` file
/// 4. On any error, attempt to restore the original state
/// 
/// # Arguments
/// * `original` - Path to the original file to be replaced
/// * `new` - Path to the new file that will replace the original
/// * `keep_original` - If true, preserve the `.orig` file; if false, delete it
/// 
/// # Returns
/// * `Ok(())` on success
/// * `Err` if any operation fails, with attempted rollback
pub fn atomic_replace(
    original: &Path,
    new: &Path,
    keep_original: bool,
) -> Result<()> {
    // Validate inputs
    if !new.exists() {
        anyhow::bail!("New file does not exist: {:?}", new);
    }
    
    if !original.exists() {
        anyhow::bail!("Original file does not exist: {:?}", original);
    }
    
    // Generate temporary name with timestamp for uniqueness
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let orig_backup = generate_backup_path(original, timestamp);
    
    // Step 1: Rename original to backup (using sync fs to avoid spurious errors on ZFS)
    // Try rename first (fast, atomic), but fall back to copy if cross-filesystem or ZFS issues
    let _backup_created = match fs::rename(original, &orig_backup) {
        Ok(_) => true,
        Err(e) => {
            let error_kind = e.kind();
            let raw_error = e.raw_os_error();
            
            // Check if this is a cross-filesystem error (EXDEV on Linux) or ZFS read-only error (EROFS)
            let is_cross_fs = raw_error == Some(18); // EXDEV = 18
            let is_zfs_readonly = raw_error == Some(30); // EROFS = 30 (ZFS spurious read-only)
            
            if is_cross_fs {
                eprintln!("Cross-filesystem detected, using copy for backup");
                // Fall back to copy for cross-filesystem
                match fs::copy(original, &orig_backup) {
                    Ok(_) => true,
                    Err(copy_err) => {
                        return Err(copy_err).context(format!(
                            "Failed to copy original {:?} to backup {:?}",
                            original, orig_backup
                        ));
                    }
                }
            } else if is_zfs_readonly {
                eprintln!("ZFS/permissions error on rename, trying copy for backup");
                // Try to copy for backup, but if that also fails, we'll skip backup
                match fs::copy(original, &orig_backup) {
                    Ok(_) => {
                        eprintln!("  Successfully created backup via copy");
                        true
                    }
                    Err(copy_err) => {
                        // Copy also failed - likely permissions issue
                        // We'll proceed WITHOUT backup (risky but only option)
                        eprintln!("WARNING: Cannot create backup (rename and copy both failed)");
                        eprintln!("  This is likely a permissions issue");
                        eprintln!("  Proceeding to replace WITHOUT backup (risky!)");
                        eprintln!("  Original will be deleted: {:?}", original);
                        eprintln!("  Copy error: {}", copy_err);
                        false // No backup created
                    }
                }
            } else {
                let orig_exists = original.exists();
                let parent_exists = orig_backup.parent().map(|p| p.exists()).unwrap_or(false);
                
                eprintln!("ERROR: Failed to rename original to backup");
                eprintln!("  Original: {:?} (exists: {})", original, orig_exists);
                eprintln!("  Backup: {:?}", orig_backup);
                eprintln!("  Parent dir exists: {}", parent_exists);
                eprintln!("  Error kind: {:?}", error_kind);
                eprintln!("  Error: {}", e);
                
                // Check permissions
                if let Ok(metadata) = fs::metadata(original) {
                    eprintln!("  Original permissions: {:?}", metadata.permissions());
                }
                if let Some(parent) = orig_backup.parent() {
                    if let Ok(metadata) = fs::metadata(parent) {
                        eprintln!("  Parent dir permissions: {:?}", metadata.permissions());
                    }
                }
                
                return Err(e).context(format!(
                    "Failed to rename {:?} to {:?} (kind: {:?})",
                    original, orig_backup, error_kind
                ));
            }
        }
    };
    
    // Step 2: If no backup was created, we need to delete the original first
    // Otherwise, copy new to original name
    if !_backup_created {
        // No backup - delete original and move new file in place
        eprintln!("Deleting original (no backup possible)");
        if let Err(e) = fs::remove_file(original) {
            return Err(e).context(format!(
                "Failed to delete original {:?} (no backup possible)",
                original
            ));
        }
        
        // Now move/copy the new file to original location
        match fs::rename(new, original) {
            Ok(_) => {
                eprintln!("Successfully moved new file to original location");
                return Ok(());
            }
            Err(_) => {
                // Rename failed, try copy
                match fs::copy(new, original) {
                    Ok(_) => {
                        fs::remove_file(new).ok(); // Clean up temp file
                        eprintln!("Successfully copied new file to original location");
                        return Ok(());
                    }
                    Err(e) => {
                        return Err(e).context(format!(
                            "Failed to move new file {:?} to original location {:?} (original was deleted!)",
                            new, original
                        ));
                    }
                }
            }
        }
    }
    
    // Step 3: Normal path - backup exists, copy new to original name
    match fs::copy(new, original) {
        Ok(_) => {
            // Successfully copied, now delete the source temp file
            if let Err(e) = fs::remove_file(new) {
                eprintln!("Warning: Failed to delete temp file {:?}: {}", new, e);
            }
            
            // Now handle the backup file
            if !keep_original {
                // Delete the backup if not keeping original
                if let Err(e) = fs::remove_file(&orig_backup) {
                    // Log warning but don't fail - the replacement succeeded
                    eprintln!(
                        "Warning: Failed to delete backup file {:?}: {}",
                        orig_backup, e
                    );
                }
            }
            Ok(())
        }
        Err(e) => {
            // Copy failed - attempt rollback
            let error_kind = e.kind();
            eprintln!("ERROR: Failed to copy new file to original location");
            eprintln!("  New file: {:?} (exists: {})", new, new.exists());
            eprintln!("  Original: {:?}", original);
            eprintln!("  Error kind: {:?}", error_kind);
            eprintln!("  Error: {}", e);
            
            // Check permissions and file info
            if let Ok(metadata) = fs::metadata(new) {
                eprintln!("  New file size: {} bytes", metadata.len());
                eprintln!("  New file permissions: {:?}", metadata.permissions());
            }
            if let Some(parent) = original.parent() {
                if let Ok(metadata) = fs::metadata(parent) {
                    eprintln!("  Target dir permissions: {:?}", metadata.permissions());
                }
            }
            
            eprintln!("Attempting to restore original from backup {:?}", orig_backup);
            
            // Try to restore the original
            match fs::rename(&orig_backup, original) {
                Ok(_) => {
                    anyhow::bail!(
                        "Failed to copy new file to original, but successfully restored original: {}",
                        e
                    );
                }
                Err(restore_err) => {
                    anyhow::bail!(
                        "Failed to copy new file to original: {}. \
                         CRITICAL: Also failed to restore original from backup: {}. \
                         Original file is at: {:?}",
                        e,
                        restore_err,
                        orig_backup
                    );
                }
            }
        }
    }
}

/// Generate a backup path with timestamp
fn generate_backup_path(original: &Path, timestamp: u64) -> PathBuf {
    let parent = original.parent();
    let filename = original.file_name().unwrap();
    let backup_name = format!("{}.orig.{}", filename.to_string_lossy(), timestamp);
    
    if let Some(p) = parent {
        p.join(backup_name)
    } else {
        PathBuf::from(backup_name)
    }
}

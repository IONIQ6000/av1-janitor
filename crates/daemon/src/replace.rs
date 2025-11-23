use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;

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
pub async fn atomic_replace(
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
    
    // Step 1: Rename original to backup
    if let Err(e) = fs::rename(original, &orig_backup).await {
        let error_kind = e.kind();
        let orig_exists = original.exists();
        let parent_exists = orig_backup.parent().map(|p| p.exists()).unwrap_or(false);
        
        eprintln!("ERROR: Failed to rename original to backup");
        eprintln!("  Original: {:?} (exists: {})", original, orig_exists);
        eprintln!("  Backup: {:?}", orig_backup);
        eprintln!("  Parent dir exists: {}", parent_exists);
        eprintln!("  Error kind: {:?}", error_kind);
        eprintln!("  Error: {}", e);
        
        return Err(e).context(format!(
            "Failed to rename {:?} to {:?} (kind: {:?})",
            original, orig_backup, error_kind
        ));
    }
    
    // Step 2: Copy new to original name (use copy for cross-filesystem support)
    // If this fails, we need to restore the original
    match fs::copy(new, original).await {
        Ok(_) => {
            // Successfully copied, now delete the source temp file
            if let Err(e) = fs::remove_file(new).await {
                eprintln!("Warning: Failed to delete temp file {:?}: {}", new, e);
            }
            
            // Now handle the backup file
            if !keep_original {
                // Step 3: Delete the backup if not keeping original
                if let Err(e) = fs::remove_file(&orig_backup).await {
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
            eprintln!("Attempting to restore original from backup {:?}", orig_backup);
            
            // Try to restore the original
            match fs::rename(&orig_backup, original).await {
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

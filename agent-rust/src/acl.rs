use std::{io, path::Path};

#[cfg(windows)]
pub fn restrict_to_admin_and_system(file_path: &Path) -> io::Result<()> {
    use winapi::um::winnt::PSID;
    use windows_acl::acl::ACL;
    use windows_acl::helper::string_to_sid;

    if !file_path.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Target path does not exist"));
    }

    let path_str = file_path
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid path encoding"))?;

    let mut acl = ACL::from_file_path(path_str, false)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("ACL init error: {}", e)))?;

    let mut system_sid = string_to_sid("S-1-5-18").unwrap();
    let mut admin_sid = string_to_sid("S-1-5-32-544").unwrap();
    let mut users_sid = string_to_sid("S-1-5-32-545").unwrap();
    let mut auth_users_sid = string_to_sid("S-1-5-11").unwrap();

    let system_psid = system_sid.as_mut_ptr() as PSID;
    let admin_psid = admin_sid.as_mut_ptr() as PSID;
    let users_psid = users_sid.as_mut_ptr() as PSID;
    let auth_users_psid = auth_users_sid.as_mut_ptr() as PSID;

    let _ = acl.remove(users_psid, None, None);
    let _ = acl.remove(auth_users_psid, None, None);

    const FILE_ALL_ACCESS: u32 = 0x001F0FFF;
    acl.allow(system_psid, true, FILE_ALL_ACCESS)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("SYSTEM grant error: {}", e)))?;

    acl.allow(admin_psid, true, FILE_ALL_ACCESS)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Admin grant error: {}", e)))?;

    Ok(())
}

#[cfg(not(windows))]
pub fn restrict_to_admin_and_system(file_path: &Path) -> io::Result<()> {
    use std::fs::set_permissions;
    use std::os::unix::fs::PermissionsExt;

    if !file_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File not found: {:?}", file_path),
        ));
    }
    set_permissions(file_path, PermissionsExt::from_mode(0o600))
}

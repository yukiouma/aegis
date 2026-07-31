use windows::{
    Win32::{
        Security::Authentication::Identity::{GetUserNameExW, NameUserPrincipal},
        System::SystemInformation::{
            COMPUTER_NAME_FORMAT, ComputerNameDnsDomain, ComputerNameDnsHostname,
            GetComputerNameExW,
        },
    },
    core::PWSTR,
};

use crate::error::{ComputerNameKind, WindowsUtilsError};

#[derive(Debug)]
pub struct DomainUserInfo {
    pub domain: String,
    pub userid: String,
    pub host_machine: String,
}

pub fn get_user_info() -> Result<DomainUserInfo, WindowsUtilsError> {
    let domain = get_domain_info()?;
    let userid_domain = get_user_id_info()?;
    let host_machine = get_host_machine_info()?;

    let userid = userid_domain
        .split("@")
        .into_iter()
        .next()
        .unwrap_or_default()
        .to_owned();

    Ok(DomainUserInfo {
        domain,
        userid,
        host_machine,
    })
}

fn get_user_id_info() -> Result<String, WindowsUtilsError> {
    let userid = NameUserPrincipal;
    let mut size: u32 = 0;
    let _ = unsafe { GetUserNameExW(userid, Some(PWSTR::null()), &mut size) };

    if size == 0 {
        return Ok(String::new());
    }

    let mut buffer = vec![0u16; size as usize];
    let result = unsafe { GetUserNameExW(userid, Some(PWSTR(buffer.as_mut_ptr())), &mut size) };

    if result {
        let name = String::from_utf16_lossy(&buffer[..size as usize]);
        Ok(name)
    } else {
        Err(WindowsUtilsError::UserName(
            windows::core::Error::from_thread(),
        ))
    }
}

fn get_domain_info() -> Result<String, WindowsUtilsError> {
    parse_computer_name(ComputerNameDnsDomain, ComputerNameKind::Domain)
}

fn get_host_machine_info() -> Result<String, WindowsUtilsError> {
    parse_computer_name(ComputerNameDnsHostname, ComputerNameKind::Hostname)
}

fn parse_computer_name(
    format: COMPUTER_NAME_FORMAT,
    kind: ComputerNameKind,
) -> Result<String, WindowsUtilsError> {
    let mut size: u32 = 0;
    let _ = unsafe { GetComputerNameExW(format, Some(PWSTR::null()), &mut size) };

    if size == 0 {
        return Ok(String::new());
    }

    let mut buffer = vec![0u16; size as usize];
    match unsafe { GetComputerNameExW(format, Some(PWSTR(buffer.as_mut_ptr())), &mut size) } {
        Ok(()) => {
            let name = String::from_utf16_lossy(&buffer[..size as usize]);
            Ok(name)
        }
        Err(source) => Err(WindowsUtilsError::ComputerName { kind, source }),
    }
}

#[cfg(test)]
mod tests {

    #[cfg(target_os = "windows")]
    mod windows_tests {
        use super::super::*;

        #[test]
        fn test_get_user_info() -> Result<(), Box<dyn std::error::Error>> {
            let info = get_user_info().unwrap();
            assert_ne!(info.domain, "");
            assert_ne!(info.userid, "");
            assert_ne!(info.host_machine, "");
            Ok(())
        }
    }
}

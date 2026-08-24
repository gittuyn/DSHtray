use crate::{app_error::AppError, domain::ServiceConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListenerOwner {
    pub pid: u32,
    pub local_address: String,
    pub port: u16,
}

pub fn find_listener(config: &ServiceConfig) -> Result<Option<ListenerOwner>, AppError> {
    #[cfg(windows)]
    {
        find_listener_windows(config)
    }

    #[cfg(not(windows))]
    {
        let _ = config;
        Err(AppError::new(
            "unsupported_platform",
            "listener 查询仅支持 Windows",
        ))
    }
}

#[cfg(windows)]
fn find_listener_windows(config: &ServiceConfig) -> Result<Option<ListenerOwner>, AppError> {
    use std::{mem::size_of, net::Ipv4Addr, slice};
    use windows::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCPTABLE_OWNER_PID, MIB_TCP_STATE_LISTEN, TCP_TABLE_OWNER_PID_ALL,
    };

    let mut size = 0_u32;
    let first_status =
        unsafe { GetExtendedTcpTable(None, &mut size, false, 2, TCP_TABLE_OWNER_PID_ALL, 0) };
    if size == 0 {
        return Err(AppError::with_details(
            "listener_query_failed",
            "无法获取 Windows TCP 表大小",
            format!("GetExtendedTcpTable 返回 {first_status}"),
        ));
    }

    let mut buffer = vec![0_u8; size as usize + size_of::<u32>()];
    let status = unsafe {
        GetExtendedTcpTable(
            Some(buffer.as_mut_ptr().cast()),
            &mut size,
            false,
            2,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        )
    };
    if status != 0 {
        return Err(AppError::with_details(
            "listener_query_failed",
            "无法读取 Windows TCP 表",
            format!("GetExtendedTcpTable 返回 {status}"),
        ));
    }

    let table = unsafe { &*(buffer.as_ptr() as *const MIB_TCPTABLE_OWNER_PID) };
    let rows = unsafe { slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize) };
    let requested_address = if config.host == "localhost" {
        "127.0.0.1"
    } else {
        config.host.as_str()
    };
    for row in rows {
        let address = Ipv4Addr::from(u32::from_be(row.dwLocalAddr));
        let port = u16::from_be(row.dwLocalPort as u16);
        if row.dwState == MIB_TCP_STATE_LISTEN.0 as u32
            && address.to_string() == requested_address
            && port == config.port
        {
            return Ok(Some(ListenerOwner {
                pid: row.dwOwningPid,
                local_address: address.to_string(),
                port,
            }));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[cfg(windows)]
    #[test]
    fn finds_pid_for_a_loopback_listener_created_by_this_test() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind listener");
        let port = listener.local_addr().expect("local address").port();
        let config = ServiceConfig {
            host: "127.0.0.1".into(),
            port,
        };
        let owner = find_listener(&config)
            .expect("listener query")
            .expect("listener should exist");
        assert_eq!(owner.pid, std::process::id());
        assert_eq!(owner.local_address, "127.0.0.1");
        assert_eq!(owner.port, port);
    }

    #[cfg(not(windows))]
    #[test]
    fn reports_unsupported_platform_outside_windows() {
        let config = ServiceConfig {
            host: "127.0.0.1".into(),
            port: 3080,
        };
        let error = find_listener(&config).expect_err("non-Windows path is unsupported");
        assert_eq!(error.code, "unsupported_platform");
    }
}
